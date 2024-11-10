use crate::{
    core::{component::ComponentId, entity::Entities, resource::ResourceId, Type},
    world::{cell::WorldCell, World},
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
        uuid::Uuid::new_v4().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct System {
    id: SystemId,
    name: Option<&'static str>,
    run: Box<dyn Fn(WorldCell) + Send + Sync>,
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

    pub fn run(&self, world: WorldCell) {
        (self.run)(world)
    }
}

pub struct SystemConfig {
    id: SystemId,
    name: Option<&'static str>,
    run: Box<dyn Fn(WorldCell) + Send + Sync>,
    access: fn() -> Vec<WorldAccess>,
    custom: Vec<WorldAccess>,
    after: Option<SystemId>,
    is_send: bool,
}

impl SystemConfig {
    pub fn new(
        name: Option<&'static str>,
        run: Box<dyn Fn(WorldCell) + Send + Sync>,
        access: fn() -> Vec<WorldAccess>,
        is_send: bool,
    ) -> Self {
        Self {
            id: SystemId::new(),
            name,
            run,
            access,
            custom: Vec::new(),
            after: None,
            is_send,
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

    pub fn add_custom(&mut self, access: WorldAccess) {
        self.custom.push(access);
    }

    pub fn after(&self) -> Option<SystemId> {
        self.after
    }

    pub fn is_send(&self) -> bool {
        self.is_send
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
    Other {
        ty: Type,
        access: AccessType,
    },
    World,
}

pub struct WorldAccessMeta {
    pub ty: Type,
    pub access: AccessType,
    pub send: bool,
}

impl From<&WorldAccess> for WorldAccessMeta {
    fn from(value: &WorldAccess) -> Self {
        match value {
            WorldAccess::Resource { ty, access, send } => Self {
                ty: ty.into(),
                access: *access,
                send: *send,
            },
            WorldAccess::Component { ty, access } => Self {
                ty: ty.into(),
                access: *access,
                send: true,
            },
            WorldAccess::Other { ty, access } => Self {
                ty: *ty,
                access: *access,
                send: true,
            },
            WorldAccess::World => Self {
                ty: Type::of::<World>(),
                access: AccessType::Read,
                send: false,
            },
        }
    }
}

impl WorldAccess {
    pub fn resource<R: crate::core::resource::Resource + Send>() -> Self {
        Self::Resource {
            ty: ResourceId::of::<R>(),
            access: AccessType::Read,
            send: true,
        }
    }

    pub fn non_send_resource<R: crate::core::resource::Resource>() -> Self {
        Self::Resource {
            ty: ResourceId::of::<R>(),
            access: AccessType::Read,
            send: false,
        }
    }

    pub fn component<C: crate::core::component::Component>() -> Self {
        Self::Component {
            ty: ComponentId::of::<C>(),
            access: AccessType::Read,
        }
    }

    pub fn world() -> Self {
        Self::World
    }

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
            WorldAccess::Other { ty, .. } => *ty,
            WorldAccess::World => Type::of::<World>(),
        }
    }

    pub fn access(&self) -> AccessType {
        match self {
            WorldAccess::Resource { access, .. } => *access,
            WorldAccess::Component { access, .. } => *access,
            WorldAccess::Other { access, .. } => *access,
            WorldAccess::World => AccessType::Read,
        }
    }

    pub fn access_ty(&self) -> (Type, AccessType, bool) {
        match self {
            WorldAccess::Resource { ty, access, send } => (ty.into(), *access, *send),
            WorldAccess::Component { ty, access } => (ty.into(), *access, false),
            WorldAccess::Other { ty, access } => (*ty, *access, false),
            WorldAccess::World => (Type::of::<World>(), AccessType::Read, true),
        }
    }

    pub fn meta(&self) -> WorldAccessMeta {
        self.into()
    }
}

pub trait SystemArg {
    type Item<'a>;

    fn init(_world: &WorldCell) {}
    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a>;
    fn access() -> Vec<WorldAccess> {
        Vec::new()
    }

    fn is_send() -> bool {
        true
    }

    fn done(_world: &WorldCell) {}
}

impl SystemArg for () {
    type Item<'a> = ();

    fn get<'a>(_: WorldCell<'a>) -> Self::Item<'a> {}
}

impl SystemArg for &World {
    type Item<'a> = &'a World;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.get()
    }

    fn is_send() -> bool {
        false
    }
}

impl SystemArg for Entities {
    type Item<'a> = &'a Entities;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.get().entities()
    }
}

pub type ArgItem<'a, A> = <A as SystemArg>::Item<'a>;

impl<F: Fn() + Send + Sync + 'static> IntoSystemConfigs<F> for F {
    fn configs(self) -> Vec<SystemConfig> {
        let name = std::any::type_name::<F>();
        let run = move |_: WorldCell| {
            self();
        };
        let access = || Vec::new();
        let is_send = true;

        vec![SystemConfig::new(
            Some(name),
            Box::new(run),
            access,
            is_send,
        )]
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
                let run = move |world: WorldCell| {
                    let ($($arg,)*) = ($($arg::get(world),)*);
                    self($($arg),*);
                };
                let access = || {
                    let mut metas = Vec::new();
                    $(metas.extend($arg::access());)*
                    metas
                };

                let is_send = ($($arg::is_send() &&)* true);

                vec![SystemConfig::new(Some(name), Box::new(run), access, is_send)]
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

            fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
                ($($arg::get(world),)*)
            }

            fn access() -> Vec<WorldAccess> {
                let mut metas = Vec::new();
                $(metas.extend($arg::access());)*
                metas
            }

            fn is_send() -> bool {
                ($($arg::is_send() &&)* true)
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

pub struct StaticArg<'w, S: SystemArg>(ArgItem<'w, S>);

impl<'w, S: SystemArg> std::ops::Deref for StaticArg<'w, S> {
    type Target = ArgItem<'w, S>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, S: SystemArg> std::ops::DerefMut for StaticArg<'w, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, S: SystemArg> StaticArg<'w, S> {
    pub fn into_inner(self) -> ArgItem<'w, S> {
        self.0
    }

    pub fn inner(&self) -> &ArgItem<'w, S> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut ArgItem<'w, S> {
        &mut self.0
    }
}

impl<S: SystemArg + 'static> SystemArg for StaticArg<'_, S> {
    type Item<'world> = StaticArg<'world, S>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        StaticArg(S::get(world))
    }

    fn access() -> Vec<WorldAccess> {
        S::access()
    }
}

pub mod unlifetime {
    use crate::{
        core::resource::{Res, ResMut},
        world::query::Query,
    };

    pub type Read<T> = &'static T;
    pub type Write<T> = &'static mut T;
    pub type ReadRes<T> = Res<'static, T>;
    pub type WriteRes<T> = ResMut<'static, T>;
    pub type StaticQuery<Q, F = ()> = Query<'static, Q, F>;
}
