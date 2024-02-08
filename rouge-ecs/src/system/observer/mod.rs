use super::{ArgItem, SystemArg};
use crate::{
    storage::{blob::Blob, sparse::SparseMap},
    tasks::{barrier::JobBarrier, ScopedTaskPool},
    world::{
        meta::{Access, AccessMeta, AccessType},
        resource::Resource,
        World,
    },
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

pub mod action;
pub mod builtin;

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

#[derive(Default)]
pub struct Observables {
    observers: SparseMap<TypeId, ObserverSystems>,
}

impl Observables {
    pub fn new() -> Self {
        Self {
            observers: SparseMap::new(),
        }
    }

    pub fn add_observer<A: Action>(&mut self, observer: Observer<A>) {
        let type_id = TypeId::of::<A>();

        if let Some(systems) = self.observers.get_mut(&type_id) {
            systems.add_observer(observer);
        } else {
            let mut systems = ObserverSystems::new::<A>();
            systems.add_observer(observer);
            self.observers.insert(type_id, systems);
        }

        self.sort();
    }

    pub fn add_observers<A: Action>(&mut self, mut observers: Observers<A>) {
        let type_id = TypeId::of::<A>();

        if let Some(systems) = self.observers.get_mut(&type_id) {
            systems.add_observers(observers.take());
        } else {
            let mut systems = ObserverSystems::new::<A>();
            systems.add_observers(observers.take());
            self.observers.insert(type_id, systems);
        }

        self.sort();
    }

    pub fn swap(&mut self, mut observables: Observables) {
        std::mem::swap(&mut self.observers, &mut observables.observers);
    }

    pub fn sort(&mut self) {
        self.observers.sort(|a, b| a.priority().cmp(&b.priority()));
    }

    pub fn execute(&mut self, mut outputs: ActionOutputs, world: &World) {
        for (type_id, observers) in self.observers.iter_mut() {
            if let Some(outputs) = outputs.remove(type_id) {
                observers.execute(outputs, world);
            }
        }
    }

    pub fn execute_actions<A: Action>(&mut self, mut outputs: ActionOutputs, world: &World) {
        if let Some(observers) = self.observers.get_mut(&TypeId::of::<A>()) {
            if let Some(outputs) = outputs.remove(&TypeId::of::<A>()) {
                observers.execute(outputs, world);
            }
        }
    }
}

pub struct Node {
    observer: ObserverSystems,
}

impl Node {
    pub fn new(observer: ObserverSystems) -> Self {
        Self { observer }
    }

    pub fn reads(&self) -> &[AccessType] {
        self.observer.reads()
    }

    pub fn writes(&self) -> &[AccessType] {
        self.observer.writes()
    }

    pub fn execute(&self, outputs: Blob, world: &World) {
        self.observer.execute(outputs, world);
    }

    pub fn type_id(&self) -> &TypeId {
        self.observer.type_id()
    }

    pub fn priority(&self) -> u32 {
        self.observer.priority()
    }

    pub fn is_local(&self) -> bool {
        self.reads().iter().any(|access| match access {
            AccessType::Local(_) | AccessType::World => true,
            _ => false,
        }) || self.writes().iter().any(|access| match access {
            AccessType::Local(_) | AccessType::World => true,
            _ => false,
        })
    }

    pub fn run(&self, world: &World, outputs: Blob) {
        self.observer.execute(outputs, world);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId {
    id: usize,
}

impl NodeId {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

impl std::ops::Deref for NodeId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

pub struct ObserverGraph {
    nodes: Vec<Node>,
    hierarchy: Vec<Vec<NodeId>>,
}

impl ObserverGraph {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            hierarchy: vec![],
        }
    }

    pub fn add_node(&mut self, observer: ObserverSystems) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node::new(observer));

        let node_id = NodeId::new(id);

        node_id
    }

    pub fn build(&mut self) {
        let mut dependency_graph = HashMap::<NodeId, HashSet<NodeId>>::new();

        for (id, node) in self.nodes.iter().enumerate() {
            for (other_id, other_node) in self.nodes.iter().enumerate() {
                let dependencies = dependency_graph.entry(NodeId::new(other_id)).or_default();
                if id == other_id || dependencies.contains(&NodeId::new(id)) {
                    continue;
                }

                let writes = node.writes();
                let reads = other_node.reads();

                let dependencies = dependency_graph.entry(NodeId::new(id)).or_default();
                if node.priority() > other_node.priority()
                    || writes.iter().any(|write| reads.contains(write))
                {
                    dependencies.insert(NodeId::new(other_id));
                }
            }
        }

        let mut hierarchy = vec![];

        while !dependency_graph.is_empty() {
            let mut group = dependency_graph
                .keys()
                .filter_map(|node_id| {
                    dependency_graph
                        .iter()
                        .all(|(_, other_dependencies)| !other_dependencies.contains(node_id))
                        .then_some(*node_id)
                })
                .collect::<Vec<NodeId>>();

            group.sort();

            for node_id in &group {
                dependency_graph.remove(node_id);
            }

            let world_nodes = group
                .iter()
                .filter_map(|node_id| {
                    self.nodes[**node_id]
                        .reads()
                        .contains(&AccessType::World)
                        .then_some(*node_id)
                })
                .collect::<Vec<_>>();

            group.retain(|node_id| !world_nodes.contains(&node_id));

            hierarchy.insert(0, group);

            for world_id in world_nodes {
                hierarchy.push(vec![world_id])
            }
        }

        hierarchy.sort_by(|a, b| {
            let a_first = a.first().unwrap();
            let b_first = b.first().unwrap();

            a_first.cmp(b_first)
        });

        self.hierarchy = hierarchy;
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn hierarchy(&self) -> &[Vec<NodeId>] {
        &self.hierarchy
    }
}

pub trait ObserverRunner: 'static {
    fn run(&self, world: &mut World, outputs: ActionOutputs);
}

pub struct SequentialRunner;

impl ObserverRunner for SequentialRunner {
    fn run(&self, world: &mut World, mut outputs: ActionOutputs) {
        let graph = world.remove_resource::<ObserverGraph>();

        for group in graph.hierarchy() {
            for node_id in group {
                let node = &graph.nodes()[**node_id];
                if let Some(outputs) = outputs.remove(&node.observer.type_id()) {
                    node.execute(outputs, world);
                }
            }
        }

        world.add_resource(graph);
    }
}

pub struct ParallelRunner;

impl ObserverRunner for ParallelRunner {
    fn run(&self, world: &mut World, mut outputs: ActionOutputs) {
        let graph = world.remove_resource::<ObserverGraph>();

        let available_threads = std::thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap())
            .into();
        for row in graph.hierarchy() {
            let local_nodes = row
                .iter()
                .filter(|id| graph.nodes()[***id].is_local())
                .collect::<Vec<_>>();
            let row = row
                .iter()
                .filter(|id| !local_nodes.contains(id))
                .collect::<Vec<_>>();
            let num_threads = row.len().min(available_threads);

            let world: &World = &world;
            ScopedTaskPool::new(num_threads, |sender| {
                let (barrier, lock) = JobBarrier::new(row.len());
                let barrier = Arc::new(Mutex::new(barrier));

                for node in &row {
                    let barrier = barrier.clone();
                    let node = &graph.nodes()[***node];

                    if let Some(outputs) = outputs.remove(&node.type_id()) {
                        sender.send(move || {
                            node.run(world, outputs);

                            barrier.lock().unwrap().notify();
                        });
                    }
                }

                sender.join();

                for node in &local_nodes {
                    let node = &graph.nodes()[***node];

                    if let Some(outputs) = outputs.remove(&node.type_id()) {
                        node.run(world, outputs);
                    }
                }

                lock.wait(barrier.lock().unwrap());
            });
        }

        world.add_resource(graph);
    }
}

impl Resource for ObserverGraph {}
impl Resource for Observables {}

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
