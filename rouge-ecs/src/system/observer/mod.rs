use super::{ArgItem, SystemArg};
use crate::{
    storage::blob::Blob,
    world::{
        meta::{AccessMeta, AccessType},
        World,
    },
};
use std::any::TypeId;

pub mod action;
pub mod builtin;
pub mod graph;

pub use action::*;

pub struct Observer<A: Action> {
    function: Box<dyn Fn(&[A::Output], &World)>,
    reads: Vec<AccessType>,
    writes: Vec<AccessType>,
}

impl<A: Action> Observer<A> {
    pub fn new(
        function: impl Fn(&[A::Output], &World) + 'static,
        reads: Vec<AccessType>,
        writes: Vec<AccessType>,
    ) -> Self {
        Self {
            function: Box::new(function),
            reads,
            writes,
        }
    }

    pub fn reads(&self) -> &[AccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[AccessType] {
        &self.writes
    }

    pub fn run(&self, outputs: &[A::Output], world: &World) {
        (self.function)(outputs, world);
    }
}

pub struct Observers<A: Action> {
    systems: Vec<Observer<A>>,
}

impl<A: Action> Observers<A> {
    pub fn new() -> Self {
        Self { systems: vec![] }
    }

    pub fn add_system<M>(mut self, system: impl IntoObserver<A, M>) -> Self {
        self.systems.push(system.into_observer());

        self
    }

    pub fn take(&mut self) -> Vec<Observer<A>> {
        std::mem::take(&mut self.systems)
    }
}

pub trait IntoObserver<A: Action, M> {
    fn into_observer(self) -> Observer<A>;
}

impl<A: Action, F> IntoObserver<A, F> for F
where
    F: Fn(&[A::Output]) + 'static,
{
    fn into_observer(self) -> Observer<A> {
        Observer::new(
            move |outputs: &[A::Output], _: &World| {
                (self)(outputs);
            },
            vec![],
            vec![],
        )
    }
}

impl<A: Action> IntoObserver<A, ()> for Observer<A> {
    fn into_observer(self) -> Observer<A> {
        self
    }
}

pub struct ObserverSystems {
    executor: Box<dyn Fn(Blob, &Blob, &World) + Send + Sync>,
    systems: Blob,
    priority: u32,
    reads: Vec<AccessType>,
    writes: Vec<AccessType>,
    type_id: TypeId,
}

impl ObserverSystems {
    pub fn new<A: Action>() -> Self {
        Self {
            executor: Box::new(move |mut outputs, systems, world| {
                let outputs = outputs.to_vec();

                for system in systems.iter_mut::<Box<Observer<A>>>() {
                    system.run(&outputs, world);
                }
            }),
            systems: Blob::new::<Box<Observer<A>>>(),
            priority: A::PRIORITY,
            reads: vec![],
            writes: vec![],
            type_id: TypeId::of::<A>(),
        }
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn add_observer<A: Action>(&mut self, observer: Observer<A>) {
        self.reads.extend(observer.reads());
        self.writes.extend(observer.writes());
        self.systems.push(Box::new(observer));
    }

    pub fn add_observers<A: Action>(&mut self, observers: Vec<Observer<A>>) {
        for observer in observers {
            self.add_observer(observer);
        }
    }

    pub fn reads(&self) -> &[AccessType] {
        &self.reads
    }

    pub fn writes(&self) -> &[AccessType] {
        &self.writes
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    pub fn execute(&self, outputs: Blob, world: &World) {
        (self.executor)(outputs, &self.systems, world);
    }
}

macro_rules! impl_into_observer {
    ($($arg:ident),*) => {
        impl<Act: Action, F, $($arg: SystemArg),*> IntoObserver<Act, (F, $($arg),*)> for F
        where
            for<'a> F: Fn(&[Act::Output], $($arg),*) + Fn(&[Act::Output], $(ArgItem<'a, $arg>),*) + 'static,
        {
            fn into_observer(self) -> Observer<Act> {
                let mut reads = vec![];
                let mut writes = vec![];
                let mut metas = vec![];

                $(metas.extend($arg::metas());)*

                AccessMeta::pick(&mut reads, &mut writes, &metas);

                let system = Observer::<Act>::new(move |outputs: &[Act::Output], world: &World| {
                    (self)(outputs, $($arg::get(world)),*);
                }, reads, writes);

                system
            }
        }
    };
}

impl_into_observer!(A);
impl_into_observer!(A, B);
impl_into_observer!(A, B, C);
impl_into_observer!(A, B, C, D);
impl_into_observer!(A, B, C, D, E);
impl_into_observer!(A, B, C, D, E, F2);
impl_into_observer!(A, B, C, D, E, F2, G);
impl_into_observer!(A, B, C, D, E, F2, G, H);
impl_into_observer!(A, B, C, D, E, F2, G, H, I);
impl_into_observer!(A, B, C, D, E, F2, G, H, I, J);
