use super::{
    world::{BaseQuery, Query, World},
    Component,
};

pub trait System: 'static {
    fn run(&self, world: &World);
}

impl<T: Fn(&World) + 'static> System for T {
    fn run(&self, world: &World) {
        self(world)
    }
}

impl<T: Fn(&World) + 'static> From<T> for Box<dyn System> {
    fn from(f: T) -> Self {
        Box::new(f)
    }
}

pub trait SystemArgFetch<'a> {
    type Item;

    fn fetch(world: &'a World) -> Self::Item;
}

pub trait SystemArg {
    type Fetch: for<'a> SystemArgFetch<'a>;
}

struct FetchWorld;
impl<'a> SystemArgFetch<'a> for FetchWorld {
    type Item = &'a World;

    fn fetch(world: &'a World) -> Self::Item {
        world
    }
}

impl SystemArg for &World {
    type Fetch = FetchWorld;
}

pub trait IntoSystemConfig<T> {
    fn config(self) -> SystemConfig;
}

impl<F, A> IntoSystemConfig<(F, A)> for F
where
    F: Fn(A) + 'static,
    A: SystemArg,
    A::Fetch: for<'a> SystemArgFetch<'a, Item = A>,
{
    fn config(self) -> SystemConfig {
        SystemConfig::new(move |world| {
            let arg = A::Fetch::fetch(world);
            (self)(arg)
        })
    }
}

pub struct SystemConfig {
    runner: Box<dyn Fn(&World)>,
}

impl SystemConfig {
    pub fn new(runner: impl Fn(&World) + 'static) -> Self {
        Self {
            runner: Box::new(runner),
        }
    }

    pub fn run(&self, world: &World) {
        (self.runner)(world)
    }
}

impl Component for i32 {}

fn test_system(w: &World) {}

fn add_system<M>(system: impl IntoSystemConfig<M>) {}

fn test() {
    add_system(test_system);
}
