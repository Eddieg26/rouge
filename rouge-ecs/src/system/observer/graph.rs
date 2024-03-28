use super::{Action, ActionOutputs, Observer, ObserverSystems, Observers};
use crate::{
    available_threads, barrier::JobBarrier, blob::Blob, meta::AccessType, resource::Resource,
    runner::RunMode, sparse::SparseMap, ScopedTaskPool, World,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

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

pub trait ObserverRunner: Send + Sync + 'static {
    fn run(&self, world: &mut World, outputs: ActionOutputs, graph: &ObserverGraph);
}

pub struct SequentialRunner;

impl ObserverRunner for SequentialRunner {
    fn run(&self, world: &mut World, mut outputs: ActionOutputs, graph: &ObserverGraph) {
        for group in graph.hierarchy() {
            for node_id in group {
                let node = &graph.nodes()[**node_id];
                if let Some(outputs) = outputs.remove(&node.observer.type_id()) {
                    node.execute(outputs, world);
                }
            }
        }
    }
}

pub struct ParallelRunner;

impl ObserverRunner for ParallelRunner {
    fn run(&self, world: &mut World, mut outputs: ActionOutputs, graph: &ObserverGraph) {
        let available_threads = available_threads();

        for row in graph.hierarchy() {
            let local_nodes = row
                .iter()
                .filter(|id| {
                    graph.nodes()[***id].is_local()
                        && outputs.contains(&graph.nodes()[***id].type_id())
                })
                .collect::<Vec<_>>();
            let row = row
                .iter()
                .filter(|id| {
                    !local_nodes.contains(id) && outputs.contains(&graph.nodes()[***id].type_id())
                })
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

                            let mut barrier_lock = barrier.lock().expect("Failed to lock barrier");
                            barrier_lock.notify();
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

                let barrier_lock = barrier.lock().expect("Failed to lock barrier");
                lock.wait(barrier_lock);
            });
        }
    }
}

pub struct Observables {
    systems: SparseMap<TypeId, ObserverSystems>,
    graph: ObserverGraph,
    built: bool,
    runner: Box<dyn ObserverRunner>,
}

impl Observables {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        let mode = RunMode::Sequential;

        #[cfg(not(target_arch = "wasm32"))]
        let mode = RunMode::Parallel;
        
        Self {
            systems: SparseMap::new(),
            graph: ObserverGraph::new(),
            built: false,
            runner: match mode {
                RunMode::Sequential => Box::new(SequentialRunner),
                RunMode::Parallel => Box::new(ParallelRunner),
            },
        }
    }

    pub fn add_observers<A: Action>(&mut self, mut observers: Observers<A>) {
        let type_id = TypeId::of::<A>();

        if let Some(systems) = self.systems.get_mut(&type_id) {
            systems.add_observers(observers.take());
        } else {
            let mut systems = ObserverSystems::new::<A>();
            systems.add_observers(observers.take());
            self.systems.insert(type_id, systems);
        }

        self.built = false;
    }

    pub fn add_observer<A: Action>(&mut self, observer: Observer<A>) {
        let type_id = TypeId::of::<A>();

        if let Some(systems) = self.systems.get_mut(&type_id) {
            systems.add_observer(observer);
        } else {
            let mut systems = ObserverSystems::new::<A>();
            systems.add_observer(observer);
            self.systems.insert(type_id, systems);
        }

        self.built = false;
    }

    pub fn build(&mut self) {
        if self.built {
            return;
        }

        for (_, systems) in self.systems.drain() {
            self.graph.add_node(systems);
        }

        self.graph.build();
        self.built = true;
    }

    pub fn run(&self, world: &mut World, outputs: ActionOutputs) {
        self.runner.run(world, outputs, &self.graph);
    }

    pub fn graph(&self) -> &ObserverGraph {
        &self.graph
    }
}

impl Resource for Observables {}
