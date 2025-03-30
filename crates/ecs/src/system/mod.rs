use crate::{
    task::ScopedTaskPool,
    world::{cell::WorldCell, World},
    Entities, NonSend, NonSendMut, Res, ResMut, Resource,
};
use hashbrown::HashMap;
use std::{
    any::TypeId,
    num::NonZero,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

pub mod observer;
pub mod schedule;

static SYSTEM_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemId(u32);
impl std::ops::Deref for SystemId {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SystemId {
    pub(crate) fn new() -> Self {
        let id = SYSTEM_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

pub type SystemFunc = Arc<dyn Fn(WorldCell) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    Exclusive,
}

pub struct SystemAccess {
    pub ty: TypeId,
    pub access: Access,
}

impl SystemAccess {
    pub fn new<T: 'static>(access: Access) -> Self {
        Self {
            ty: TypeId::of::<T>(),
            access,
        }
    }

    pub fn world() -> Self {
        Self {
            ty: TypeId::of::<World>(),
            access: Access::Exclusive,
        }
    }
}

pub struct SystemConfig {
    id: SystemId,
    name: Option<&'static str>,
    access: Vec<SystemAccess>,
    after: Vec<SystemId>,
    send: bool,
    exclusive: bool,
    system: SystemFunc,
}

impl SystemConfig {
    pub fn new(
        name: Option<&'static str>,
        system: SystemFunc,
        access: Vec<SystemAccess>,
        send: bool,
    ) -> Self {
        let exclusive = access
            .iter()
            .any(|access| access.access == Access::Exclusive);

        Self {
            id: SystemId::new(),
            name,
            system,
            access,
            after: Vec::new(),
            send,
            exclusive,
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn access(&self) -> &[SystemAccess] {
        &self.access
    }

    pub fn after(&self) -> &[SystemId] {
        &self.after
    }

    pub fn send(&self) -> bool {
        self.send
    }

    pub fn exclusive(&self) -> bool {
        self.exclusive
    }
}

pub struct System {
    id: SystemId,
    name: Option<&'static str>,
    system: SystemFunc,
}

impl System {
    pub fn new(config: SystemConfig) -> Self {
        Self {
            id: config.id,
            name: config.name,
            system: config.system,
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn run(&self, world: WorldCell) {
        (self.system)(world);
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

    fn before<Marker>(mut self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        self.after.extend(configs.iter().map(|s| s.id));

        configs.push(self);
        configs
    }

    fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        configs.iter_mut().for_each(|s| s.after.push(self.id));

        configs.push(self);
        configs
    }
}

impl<M, I: IntoSystemConfigs<M>> IntoSystemConfigs<M> for Vec<I> {
    fn configs(self) -> Vec<SystemConfig> {
        self.into_iter().flat_map(|i| i.configs()).collect()
    }

    fn before<Marker>(mut self, configs: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = configs.configs();
        for before in self.drain(..).map(|s| s.configs()) {
            for mut config in before {
                config.after.extend(configs.iter().map(|s| s.id));
                configs.push(config);
            }
        }

        configs
    }

    fn after<Marker>(mut self, configs: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = configs.configs();
        for after in self.drain(..).map(|s| s.configs()) {
            for config in after {
                configs.iter_mut().for_each(|s| s.after.push(config.id));
                configs.push(config);
            }
        }

        configs
    }
}

impl<F: Fn() + Send + Sync + 'static> IntoSystemConfigs<F> for F {
    fn configs(self) -> Vec<SystemConfig> {
        let name = std::any::type_name::<F>();
        let run = move |_: WorldCell| {
            self();
        };

        vec![SystemConfig::new(Some(name), Arc::new(run), vec![], true)]
    }

    fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        system.after(self)
    }

    fn after<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        let config = self.configs().pop().unwrap();

        configs.iter_mut().for_each(|s| s.after.push(config.id));
        configs.push(config);

        configs
    }
}

pub enum SystemCluster {
    Parallel(Vec<usize>),
    Sequential(Vec<usize>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Sequential,
    Parallel,
}

pub struct SystemGraph {
    clusters: Vec<SystemCluster>,
    systems: Vec<System>,
}

impl SystemGraph {
    pub fn new(mut configs: Vec<SystemConfig>) -> Self {
        let mut dependencies = HashMap::new();
        for (index, config) in configs.iter().enumerate() {
            for after in &config.after {
                dependencies
                    .entry(*after)
                    .or_insert_with(Vec::new)
                    .push(config.id);
            }

            dependencies.entry(config.id).or_insert_with(Vec::new);
            for other in configs.iter().skip(index + 1) {
                if config.access.iter().any(|access| {
                    other.access.iter().any(|other| {
                        access.ty == other.ty
                            && (access.access == Access::Write || other.access == Access::Write)
                    })
                }) {
                    dependencies.entry(other.id).or_default().push(config.id);
                }
            }
        }

        let mut clusters = Vec::new();

        while !dependencies.is_empty() {
            let group = dependencies
                .iter()
                .filter_map(|(id, deps)| {
                    deps.iter()
                        .all(|dep| !dependencies.contains_key(dep))
                        .then_some(*id)
                })
                .collect::<Vec<_>>();

            if group.is_empty() {
                panic!("Cyclic dependency detected");
            }

            let current: Vec<SystemCluster> = vec![];
            let cluster = group
                .iter()
                .fold(current, |mut current, id| match current.last_mut() {
                    Some(cluster) => {
                        let index = configs.iter().position(|config| config.id == *id).unwrap();
                        let config = &configs[index];
                        let sequintial = config.exclusive || !config.send;

                        match (sequintial, cluster) {
                            (true, SystemCluster::Sequential(items)) => items.push(index),
                            (false, SystemCluster::Parallel(items)) => items.push(index),
                            (true, _) => {
                                let cluster = SystemCluster::Sequential(vec![index]);
                                current.push(cluster);
                            }
                            (false, _) => {
                                let cluster = SystemCluster::Parallel(vec![index]);
                                current.push(cluster);
                            }
                        }

                        dependencies.remove(id);

                        current
                    }
                    None => {
                        let index = configs.iter().position(|config| config.id == *id).unwrap();
                        let config = &configs[index];
                        if config.exclusive || !config.send {
                            current.push(SystemCluster::Sequential(vec![index]));
                        } else {
                            current.push(SystemCluster::Parallel(vec![index]));
                        }

                        dependencies.remove(id);

                        current
                    }
                });

            clusters.extend(cluster);
        }

        Self {
            clusters,
            systems: configs.drain(..).map(|c| System::new(c)).collect(),
        }
    }

    pub fn run(&self, world: &World, mode: RunMode) {
        let world = unsafe { world.cell() };
        match mode {
            RunMode::Sequential => {
                for system in &self.systems {
                    system.run(world);
                }
            }
            RunMode::Parallel => {
                for cluster in &self.clusters {
                    match cluster {
                        SystemCluster::Parallel(items) => {
                            let pool_size = items.len().min(Self::max_threads());
                            if pool_size > 0 {
                                let mut pool = ScopedTaskPool::new(pool_size);
                                for index in items.iter() {
                                    pool.spawn(move || self.systems[*index].run(world));
                                }

                                pool.run();
                            }
                        }
                        SystemCluster::Sequential(items) => {
                            for index in items {
                                self.systems[*index].run(world);
                            }
                        }
                    }
                }
            }
        }
    }

    fn max_threads() -> usize {
        std::thread::available_parallelism()
            .unwrap_or(NonZero::<usize>::new(1).unwrap())
            .into()
    }
}

pub trait SystemArg {
    type Item<'a>;

    fn init(_world: &WorldCell) {}
    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a>;
    fn access() -> Vec<SystemAccess> {
        Vec::new()
    }

    fn send() -> bool {
        true
    }

    fn validate(_world: &WorldCell) -> bool {
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

    fn send() -> bool {
        false
    }
}

impl SystemArg for &Entities {
    type Item<'a> = &'a Entities;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.get().entities()
    }
}

impl<R: Resource + Send> SystemArg for Option<Res<'_, R>> {
    type Item<'a> = Option<Res<'a, R>>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.try_resource::<R>()
    }
}

impl<R: Resource + Send> SystemArg for Option<ResMut<'_, R>> {
    type Item<'a> = Option<ResMut<'a, R>>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.try_resource_mut::<R>()
    }
}

impl<R: Resource> SystemArg for Option<NonSend<'_, R>> {
    type Item<'a> = Option<NonSend<'a, R>>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.try_non_send_resource::<R>()
    }
}

impl<R: Resource> SystemArg for Option<NonSendMut<'_, R>> {
    type Item<'a> = Option<NonSendMut<'a, R>>;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.try_non_send_resource_mut::<R>()
    }
}

pub type ArgItem<'a, A> = <A as SystemArg>::Item<'a>;

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
                let mut access = Vec::new();
                $(access.extend($arg::access());)*

                let send = ($($arg::send() &&)* true);

                vec![SystemConfig::new(Some(name), Arc::new(run), access, send)]
            }

            fn before<Marker>(self, configs: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
                configs.after(self)
            }

            fn after<Marker>(self, configs: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
                let mut configs = configs.configs();
                let config = self.configs().pop().unwrap();

                configs.iter_mut().for_each(|s| s.after.push(config.id));
                configs.push(config);

                configs
            }
        }

        impl<$($arg: SystemArg),*> SystemArg for ($($arg,)*) {
            type Item<'a> = ($($arg::Item<'a>,)*);

            fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
                ($($arg::get(world),)*)
            }

            fn access() -> Vec<SystemAccess> {
                let mut metas = Vec::new();
                $(metas.extend($arg::access());)*
                metas
            }

            fn send() -> bool {
                ($($arg::send() &&)* true)
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

impl<'w, 's, S: SystemArg> AsRef<ArgItem<'w, S>> for StaticArg<'w, S> {
    fn as_ref(&self) -> &ArgItem<'w, S> {
        &self.0
    }
}

impl<'w, 's, S: SystemArg> AsMut<ArgItem<'w, S>> for StaticArg<'w, S> {
    fn as_mut(&mut self) -> &mut ArgItem<'w, S> {
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

    fn access() -> Vec<SystemAccess> {
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
