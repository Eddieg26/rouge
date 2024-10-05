use crate::{
    core::{entity::Entities, registry::Type, resource::ResourceId},
    world::{components::ComponentId, World},
};
use std::hash::Hash;

pub mod observer;
pub mod schedule;
pub mod systems;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(u32);
impl SystemId {
    pub fn new() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        ulid::Ulid::new().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct System {
    id: SystemId,
    name: Option<&'static str>,
    run: Box<dyn Fn(&World) + Sync>,
}

impl System {
    pub fn new(config: SystemConfig) -> Self {
        Self {
            id: config.id,
            name: config.name,
            run: config.run,
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn run(&self, world: &World) {
        (self.run)(world)
    }
}

pub struct SystemConfig {
    id: SystemId,
    name: Option<&'static str>,
    run: Box<dyn Fn(&World) + Sync>,
    access: fn() -> Vec<WorldAccess>,
    after: Option<SystemId>,
}

impl SystemConfig {
    pub fn new(
        name: Option<&'static str>,
        run: Box<dyn Fn(&World) + Sync>,
        access: fn() -> Vec<WorldAccess>,
    ) -> Self {
        Self {
            id: SystemId::new(),
            name,
            run,
            access,
            after: None,
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn access(&self) -> Vec<WorldAccess> {
        (self.access)()
    }
}

impl From<SystemConfig> for System {
    fn from(config: SystemConfig) -> Self {
        System::new(config)
    }
}

pub trait IntoSystemConfigs<M> {
    fn configs(self) -> Vec<SystemConfig>;
    fn before<Marker>(self, systems: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig>;
    fn after<Marker>(self, systems: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig>;
}

impl IntoSystemConfigs<()> for SystemConfig {
    fn configs(self) -> Vec<SystemConfig> {
        vec![self]
    }

    fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        system.after(self)
    }

    fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        let id = configs.first().unwrap().id();

        let mut config = self;
        config.after = Some(id);

        match configs.iter().position(|config| config.id() == id) {
            Some(index) => configs.insert(index + 1, config),
            None => configs.push(config),
        }

        configs
    }
}

//
impl<M, I: IntoSystemConfigs<M>> IntoSystemConfigs<M> for Vec<I> {
    fn configs(self) -> Vec<SystemConfig> {
        self.into_iter()
            .flat_map(|config| config.configs())
            .collect()
    }

    fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        system.after(self)
    }

    fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        let id = configs.last().unwrap().id();

        for config in self.into_iter().flat_map(|config| config.configs()) {
            let mut config = config;
            config.after = Some(id);
            configs.push(config);
        }

        configs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessType {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldAccess {
    Resource {
        ty: ResourceId,
        access: AccessType,
        send: bool,
    },
    Component {
        ty: ComponentId,
        access: AccessType,
    },
    World,
}

impl WorldAccess {
    pub fn types_equal(&self, other: &WorldAccess) -> bool {
        match (self, other) {
            (WorldAccess::Resource { ty: a, .. }, WorldAccess::Resource { ty: b, .. }) => a == b,
            (WorldAccess::Component { ty: a, .. }, WorldAccess::Component { ty: b, .. }) => a == b,
            (WorldAccess::World, WorldAccess::World) => true,
            _ => false,
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            WorldAccess::Resource { ty, .. } => ty.into(),
            WorldAccess::Component { ty, .. } => ty.into(),
            WorldAccess::World => Type::of::<World>(),
        }
    }

    pub fn access(&self) -> AccessType {
        match self {
            WorldAccess::Resource { access, .. } => *access,
            WorldAccess::Component { access, .. } => *access,
            WorldAccess::World => AccessType::Read,
        }
    }

    pub fn access_ty(&self) -> (Type, AccessType, bool) {
        match self {
            WorldAccess::Resource { ty, access, send } => (ty.into(), *access, !send),
            WorldAccess::Component { ty, access } => (ty.into(), *access, false),
            WorldAccess::World => (Type::of::<World>(), AccessType::Read, true),
        }
    }
}

pub trait SystemArg {
    type Item<'a>;

    fn get<'a>(world: &'a World) -> Self::Item<'a>;
    fn access() -> Vec<WorldAccess> {
        Vec::new()
    }
}

impl SystemArg for () {
    type Item<'a> = ();

    fn get<'a>(_: &'a World) -> Self::Item<'a> {}
}

impl SystemArg for &World {
    type Item<'a> = &'a World;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world
    }
}

impl SystemArg for Entities {
    type Item<'a> = &'a Entities;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.entities()
    }
}

pub type ArgItem<'a, A> = <A as SystemArg>::Item<'a>;

impl<F: Fn() + Send + Sync + 'static> IntoSystemConfigs<F> for F {
    fn configs(self) -> Vec<SystemConfig> {
        let name = std::any::type_name::<F>();
        let run = move |_: &World| {
            self();
        };
        let access = || Vec::new();

        vec![SystemConfig::new(Some(name), Box::new(run), access)]
    }

    fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        system.after(self)
    }

    fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        let id = configs.first().unwrap().id();

        let mut config = self.configs().pop().unwrap();
        config.after = Some(id);

        match configs.iter().position(|config| config.id() == id) {
            Some(index) => configs.insert(index + 1, config),
            None => configs.push(config),
        }

        configs
    }
}

macro_rules! impl_into_system_configs {
    ($($arg:ident),*) => {
    #[allow(non_snake_case)]
    impl<F, $($arg: SystemArg),*> IntoSystemConfigs<(F, $($arg),*)> for F
        where
            for<'a> F: Fn($($arg),*) + Fn($(ArgItem<'a, $arg>),*) + Send + Sync + 'static,
        {
            fn configs(self) -> Vec<SystemConfig> {
                let name = std::any::type_name::<F>();
                let run = move |world: &World| {
                    let ($($arg,)*) = ($($arg::get(world),)*);
                    self($($arg),*);
                };
                let access = || {
                    let mut metas = Vec::new();
                    $(metas.extend($arg::access());)*
                    metas
                };

                vec![SystemConfig::new(Some(name), Box::new(run), access)]
            }

            fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
                system.after(self)
            }

            fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
                let mut configs = system.configs();
                let id = configs.first().unwrap().id();

                let mut config = self.configs().pop().unwrap();
                config.after = Some(id);

                match configs.iter().position(|config| config.id() == id) {
                    Some(index) => configs.insert(index + 1, config),
                    None => configs.push(config),
                }

                configs
            }
        }

        impl<$($arg: SystemArg),*> SystemArg for ($($arg,)*) {
            type Item<'a> = ($($arg::Item<'a>,)*);

            fn get<'a>(world: &'a World) -> Self::Item<'a> {
                ($($arg::get(world),)*)
            }

            fn access() -> Vec<WorldAccess> {
                let mut metas = Vec::new();
                $(metas.extend($arg::access());)*
                metas
            }
        }
    };
}

impl_into_system_configs!(A);
impl_into_system_configs!(A, B);
impl_into_system_configs!(A, B, C);
impl_into_system_configs!(A, B, C, D);
impl_into_system_configs!(A, B, C, D, E);
impl_into_system_configs!(A, B, C, D, E, F2);
impl_into_system_configs!(A, B, C, D, E, F2, G);
impl_into_system_configs!(A, B, C, D, E, F2, G, H);
impl_into_system_configs!(A, B, C, D, E, F2, G, H, I);
impl_into_system_configs!(A, B, C, D, E, F2, G, H, I, J);
