use crate::{
    core::Entities,
    world::{
        meta::{Access, AccessMeta, AccessType},
        resource::{LocalResource, Resource},
        World,
    },
};

pub mod observer;

pub struct System {
    function: Box<dyn for<'a> Fn(&'a World) + Send + Sync>,
    reads: Vec<AccessType>,
    writes: Vec<AccessType>,
    before: Vec<System>,
    after: Vec<System>,
}

impl System {
    pub fn new<F>(function: F, reads: Vec<AccessType>, writes: Vec<AccessType>) -> Self
    where
        F: for<'a> Fn(&'a World) + Send + Sync + 'static,
    {
        Self {
            function: Box::new(function),
            reads,
            writes,
            before: vec![],
            after: vec![],
        }
    }

    pub fn reads(&self) -> &[AccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[AccessType] {
        &self.writes
    }

    pub(crate) fn befores_mut(&mut self) -> &mut Vec<System> {
        &mut self.before
    }

    pub(crate) fn afters_mut(&mut self) -> &mut Vec<System> {
        &mut self.after
    }

    pub fn run(&self, world: &World) {
        (self.function)(world);
    }
}

impl IntoSystem<()> for System {
    fn into_system(self) -> System {
        self
    }

    fn before<Marker>(mut self, system: impl IntoSystem<Marker>) -> System {
        self.before.push(system.into_system());
        self
    }

    fn after<Marker>(mut self, system: impl IntoSystem<Marker>) -> System {
        self.after.push(system.into_system());
        self
    }
}

/// A collection of systems that can be run in sequence.
pub struct SystemSet {
    systems: Vec<System>,
}

impl SystemSet {
    pub fn new() -> Self {
        Self { systems: vec![] }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystem<M>) {
        self.systems.push(system.into_system());
    }

    pub fn append(&mut self, mut system_set: SystemSet) {
        self.systems.append(&mut system_set.systems);
    }

    pub fn reads(&self) -> Vec<AccessType> {
        self.systems
            .iter()
            .flat_map(|system| system.reads().to_vec())
            .collect()
    }

    pub fn writes(&self) -> Vec<AccessType> {
        self.systems
            .iter()
            .flat_map(|system| system.writes().to_vec())
            .collect()
    }
}

impl IntoSystem<()> for SystemSet {
    fn into_system(self) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system
    }

    fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let mut system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system.before.push(other.into_system());

        system
    }

    fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut reads = vec![];
        let mut writes = vec![];

        for system in &self.systems {
            reads.extend(system.reads().to_vec());
            writes.extend(system.writes().to_vec());
        }

        let mut system = System::new(
            move |world| {
                for system in &self.systems {
                    system.run(world);
                }
            },
            reads,
            writes,
        );

        system.after.push(other.into_system());

        system
    }
}

pub trait SystemArg {
    type Item<'a>;

    fn get<'a>(world: &'a World) -> Self::Item<'a>;
    fn metas() -> Vec<AccessMeta>;
}

impl SystemArg for () {
    type Item<'a> = ();

    fn get<'a>(_: &'a World) -> Self::Item<'a> {}

    fn metas() -> Vec<AccessMeta> {
        vec![]
    }
}

pub type ArgItem<'a, A> = <A as SystemArg>::Item<'a>;

pub trait IntoSystem<M> {
    fn into_system(self) -> System;
    fn before<Marker>(self, system: impl IntoSystem<Marker>) -> System;
    fn after<Marker>(self, system: impl IntoSystem<Marker>) -> System;
}

pub trait IntoSystems<M> {
    fn into_systems(self) -> Vec<System>;
}

impl SystemArg for &World {
    type Item<'a> = &'a World;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::world();
        vec![AccessMeta::new(ty, Access::Read)]
    }
}

impl<R: Resource> SystemArg for &R {
    type Item<'a> = &'a R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.resource::<R>()
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::resource::<R>();
        vec![AccessMeta::new(ty, Access::Read)]
    }
}

impl<R: Resource> SystemArg for &mut R {
    type Item<'a> = &'a mut R;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.resource_mut::<R>()
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::resource::<R>();
        vec![AccessMeta::new(ty, Access::Write)]
    }
}

impl<R: Resource> SystemArg for Option<&R> {
    type Item<'a> = Option<&'a R>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.try_resource::<R>()
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::resource::<R>();
        vec![AccessMeta::new(ty, Access::Read)]
    }
}

impl<R: Resource> SystemArg for Option<&mut R> {
    type Item<'a> = Option<&'a mut R>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.try_resource_mut::<R>()
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::resource::<R>();
        vec![AccessMeta::new(ty, Access::Write)]
    }
}

pub struct Local<'a, R: LocalResource> {
    resource: &'a R,
    _marker: std::marker::PhantomData<R>,
}

impl<'a, R: LocalResource> Local<'a, R> {
    pub fn new(resource: &'a R) -> Self {
        Self {
            resource,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, R: LocalResource> std::ops::Deref for Local<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.resource
    }
}

pub struct LocalMut<'a, R: LocalResource> {
    resource: &'a mut R,
    _marker: std::marker::PhantomData<R>,
}

impl<'a, R: LocalResource> LocalMut<'a, R> {
    pub fn new(resource: &'a mut R) -> Self {
        Self {
            resource,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, R: LocalResource> std::ops::Deref for LocalMut<'a, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.resource
    }
}

impl<'a, R: LocalResource> std::ops::DerefMut for LocalMut<'a, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resource
    }
}

impl<R: LocalResource> SystemArg for Local<'_, R> {
    type Item<'a> = Local<'a, R>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        let resource = world.local_resource::<R>();
        Local::new(resource)
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::local_resource::<R>();
        vec![AccessMeta::new(ty, Access::Read)]
    }
}

impl<R: LocalResource> SystemArg for LocalMut<'_, R> {
    type Item<'a> = LocalMut<'a, R>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        let resource = world.local_resource_mut::<R>();
        LocalMut::new(resource)
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::local_resource::<R>();
        vec![AccessMeta::new(ty, Access::Write)]
    }
}

impl<R: LocalResource> SystemArg for Option<Local<'_, R>> {
    type Item<'a> = Option<Local<'a, R>>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        let resource = world.try_local_resource::<R>();
        resource.map(Local::new)
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::local_resource::<R>();
        vec![AccessMeta::new(ty, Access::Read)]
    }
}

impl<R: LocalResource> SystemArg for Option<LocalMut<'_, R>> {
    type Item<'a> = Option<LocalMut<'a, R>>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        let resource = world.try_local_resource_mut::<R>();
        resource.map(LocalMut::new)
    }

    fn metas() -> Vec<AccessMeta> {
        let ty = AccessType::local_resource::<R>();
        vec![AccessMeta::new(ty, Access::Write)]
    }
}

pub struct Cloned<R: Resource + Clone> {
    resource: R,
    _marker: std::marker::PhantomData<R>,
}

impl<R: Resource + Clone> SystemArg for Cloned<R> {
    type Item<'a> = Cloned<R>;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        let resource = world.resource::<R>().clone();
        Cloned {
            resource,
            _marker: std::marker::PhantomData,
        }
    }

    fn metas() -> Vec<AccessMeta> {
        vec![]
    }
}

impl<R: Resource + Clone> std::ops::Deref for Cloned<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<R: Resource + Clone> std::ops::DerefMut for Cloned<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource
    }
}

impl SystemArg for &Entities {
    type Item<'a> = &'a Entities;

    fn get<'a>(world: &'a World) -> Self::Item<'a> {
        world.entities()
    }

    fn metas() -> Vec<AccessMeta> {
        vec![AccessMeta::new(AccessType::None, Access::Read)]
    }
}

impl<F: Fn() + Send + Sync + 'static> IntoSystem<F> for F {
    fn into_system(self) -> System {
        let system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system
    }

    fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system.before.push(other.into_system());

        system
    }

    fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
        let mut system = System::new(
            move |_| {
                (self)();
            },
            vec![],
            vec![],
        );

        system.after.push(other.into_system());

        system
    }
}

macro_rules! impl_into_system {
    ($($arg:ident),*) => {
        impl<F, $($arg: SystemArg),*> IntoSystem<(F, $($arg),*)> for F
        where
            for<'a> F: Fn($($arg),*) + Fn($(ArgItem<'a, $arg>),*) + Send + Sync + 'static,
        {
            fn into_system(self) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::metas());)*

                AccessMeta::pick(&mut reads, &mut writes, &metas);

                let system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system
            }

            fn before<Marker>(self, other: impl IntoSystem<Marker>) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::metas());)*

                AccessMeta::pick(&mut reads, &mut writes, &metas);

                let mut system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system.before.push(other.into_system());

                system
            }

            fn after<Marker>(self, other: impl IntoSystem<Marker>) -> System {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::metas());)*

                AccessMeta::pick(&mut reads, &mut writes, &metas);

                let mut system = System::new(move |world| {
                    (self)($($arg::get(world)),*);
                }, reads, writes);

                system.after.push(other.into_system());

                system
            }
        }

        impl<$($arg: SystemArg),*> SystemArg for ($($arg,)*) {
            type Item<'a> = ($($arg::Item<'a>,)*);

            fn get<'a>(world: &'a World) -> Self::Item<'a> {
                ($($arg::get(world),)*)
            }

            fn metas() -> Vec<AccessMeta> {
                let mut metas = Vec::new();
                $(metas.extend($arg::metas());)*
                metas
            }
        }
    };
}

impl_into_system!(A);
impl_into_system!(A, B);
impl_into_system!(A, B, C);
impl_into_system!(A, B, C, D);
impl_into_system!(A, B, C, D, E);
impl_into_system!(A, B, C, D, E, F2);
impl_into_system!(A, B, C, D, E, F2, G);
impl_into_system!(A, B, C, D, E, F2, G, H);
impl_into_system!(A, B, C, D, E, F2, G, H, I);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
// impl_into_system!(A, B, C, D, E, F2, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
