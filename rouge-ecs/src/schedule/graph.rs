use crate::{
    system::System,
    world::{meta::AccessType, World},
};
use std::{
    collections::{HashMap, HashSet},
    vec,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl std::ops::Deref for NodeId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for NodeId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Node {
    system: System,
    dependencies: Vec<NodeId>,
}

impl Node {
    pub fn new(system: System) -> Self {
        Self {
            system,
            dependencies: vec![],
        }
    }

    pub fn run(&self, world: &World) {
        self.system.run(world);
    }

    pub fn reads(&self) -> &[AccessType] {
        self.system.reads()
    }

    pub fn writes(&self) -> &[AccessType] {
        self.system.writes()
    }

    pub fn dependencies(&self) -> &[NodeId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, dependency: NodeId) {
        self.dependencies.push(dependency);
    }

    pub fn is_local(&self) -> bool {
        self.system.reads().iter().any(|access| match access {
            AccessType::Local(_) | AccessType::World => true,
            _ => false,
        }) || self.system.writes().iter().any(|access| match access {
            AccessType::Local(_) | AccessType::World => true,
            _ => false,
        })
    }
}

pub struct SystemGraph {
    nodes: Vec<Node>,
    hierarchy: Vec<Vec<NodeId>>,
}

impl SystemGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            hierarchy: Vec::new(),
        }
    }

    pub fn add_system(&mut self, mut system: System) -> NodeId {
        let mut before_systems = std::mem::take(system.befores_mut());

        let after_ids = system
            .afters_mut()
            .drain(..)
            .into_iter()
            .map(|system| self.add_system(system))
            .collect::<Vec<_>>();

        let node = Node::new(system);
        let node_id = self.add_node(node);

        for after_id in after_ids {
            self.nodes[*after_id].add_dependency(node_id);
        }

        let before_ids = before_systems
            .drain(..)
            .into_iter()
            .map(|system| self.add_system(system))
            .collect::<Vec<_>>();

        for before_id in before_ids {
            self.nodes[*node_id].add_dependency(before_id);
        }

        node_id
    }

    fn add_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);

        id
    }

    pub fn append(&mut self, other: &mut Self) {
        let offset = self.nodes.len();

        self.nodes.append(&mut other.nodes);

        for parents in &mut other.hierarchy {
            for parent in parents {
                parent.0 += offset;
            }
        }
    }

    pub fn reads(&self) -> Vec<AccessType> {
        self.nodes
            .iter()
            .flat_map(|node| node.reads().to_vec())
            .collect()
    }

    pub fn writes(&self) -> Vec<AccessType> {
        self.nodes
            .iter()
            .flat_map(|node| node.writes().to_vec())
            .collect()
    }

    pub fn build(&mut self) {
        let mut dependency_graph = HashMap::<NodeId, HashSet<NodeId>>::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for (j, other_node) in self.nodes.iter().enumerate() {
                let dependencies = dependency_graph.entry(NodeId::new(j)).or_default();
                if i == j || dependencies.contains(&NodeId::new(i)) {
                    continue;
                }

                let writes = node.writes();
                let reads = other_node.reads();

                let dependencies = dependency_graph.entry(NodeId::new(i)).or_default();
                if writes
                    .iter()
                    .any(|write| (*write) != AccessType::None && reads.contains(write))
                {
                    dependencies.insert(NodeId::new(j));
                }
            }

            let dependencies = dependency_graph.entry(NodeId::new(i)).or_default();
            for dependency in node.dependencies() {
                dependencies.insert(*dependency);
            }
        }

        let mut hierarchy = Vec::new();

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

        hierarchy.sort_by(|a, b| match (a.first(), b.first()) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
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
