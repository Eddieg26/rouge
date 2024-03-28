use self::{
    context::RenderContext,
    nodes::{GraphNode, NodeId, RenderPhase},
    resources::{GraphResources, TextureDesc},
};
use crate::{
    core::device::RenderDevice,
    resources::{buffer::BaseBuffer, texture::GpuTexture},
};
use rouge_core::ResourceId;
use rouge_ecs::{
    macros::Resource,
    meta::{Access, AccessMeta, AccessType},
    runner::RunMode,
    sparse::SparseMap,
    World,
};
use std::collections::{HashMap, HashSet};

pub mod context;
pub mod nodes;
pub mod resources;

#[derive(Resource)]
pub struct RenderGraph {
    resources: GraphResources,
    nodes: SparseMap<NodeId, Box<dyn GraphNode>>,
    hierarchy: Vec<Vec<NodeId>>,
    mode: RunMode,
}

impl RenderGraph {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        let mode = RunMode::Sequential;

        #[cfg(not(target_arch = "wasm32"))]
        let mode = RunMode::Parallel;

        Self {
            resources: GraphResources::new(),
            nodes: SparseMap::new(),
            hierarchy: Vec::new(),
            mode,
        }
    }

    pub fn node<N: GraphNode>(&self, id: impl Into<NodeId>) -> Option<&N> {
        let id = id.into();
        self.nodes.get(&id).map(|n| n.downcast_ref::<N>().unwrap())
    }

    pub fn node_mut<N: GraphNode>(&mut self, id: NodeId) -> Option<&mut N> {
        self.nodes
            .get_mut(&id)
            .map(|n| n.downcast_mut::<N>().unwrap())
    }

    pub fn hierarchy(&self) -> &[Vec<NodeId>] {
        &self.hierarchy
    }

    pub fn add_node<N: GraphNode>(&mut self, id: impl Into<NodeId>, node: N) -> &mut Self {
        let id = id.into();
        assert!(
            !self.nodes.contains(&id),
            "Node with id {:?} already exists",
            id
        );

        self.nodes.insert(id, Box::new(node));
        self
    }

    pub fn import_texture(&mut self, id: impl Into<ResourceId>, texture: GpuTexture) -> &mut Self {
        self.resources.import_texture(id, texture);
        self
    }

    pub fn import_buffer(
        &mut self,
        id: impl Into<ResourceId>,
        buffer: impl BaseBuffer,
    ) -> &mut Self {
        self.resources.import_buffer(id, buffer);
        self
    }

    pub fn create_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) -> ResourceId {
        self.resources.create_texture(id, desc)
    }

    pub fn build(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.resources.build(device, width, height);
        self.build_hierarchy();
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.resources.build(device, width, height);
    }

    pub fn execute(&self, world: &World) {
        let ctx = RenderContext::new(world, &self.resources, world.resource::<RenderDevice>());

        for ids in &self.hierarchy {
            for id in ids {
                let node = &self.nodes[id];
                node.execute(ctx.clone());
            }
        }
    }

    pub fn build_hierarchy(&mut self) {
        let mut dependencies = HashMap::<NodeId, HashSet<NodeId>>::new();

        for (index, (id, node)) in self.nodes.iter().enumerate() {
            for (other_index, (other_id, other_node)) in self.nodes.iter().enumerate() {
                let (deps, possible_dep) = if index > other_index {
                    (dependencies.entry(*id).or_default(), other_id)
                } else {
                    (dependencies.entry(*other_id).or_default(), id)
                };

                if id == other_id || deps.contains(other_id) || node.phase() != other_node.phase() {
                    continue;
                }

                let access = node.access();
                let other_access = other_node.access();

                if access.iter().any(|meta| match meta.access() {
                    Access::Read => other_access.iter().any(|other_meta| {
                        other_meta.access() == Access::Write
                            && other_meta.ty() == meta.ty()
                            && meta.ty() != AccessType::None
                    }),
                    Access::Write => other_access.contains(meta),
                }) {
                    deps.insert(*possible_dep);
                }
            }
        }

        let mut hierarchy = Vec::new();

        for phase in RenderPhase::iter() {
            while dependencies
                .iter()
                .any(|(id, _)| self.nodes[id].phase() == phase)
            {
                let mut next = Vec::new();
                for (id, deps) in dependencies.iter() {
                    if deps.is_empty() && self.nodes[id].phase() == phase {
                        next.push(*id);
                    }
                }

                if next.is_empty() {
                    panic!("Cyclic dependency detected: {:?}", phase);
                }

                for id in &next {
                    dependencies.remove(id);
                }

                for (_, deps) in dependencies.iter_mut() {
                    deps.retain(|id| !next.contains(id));
                }

                match self.mode {
                    RunMode::Sequential => hierarchy.push(next),
                    RunMode::Parallel => {
                        let world_ids = next
                            .iter()
                            .filter_map(|id| {
                                self.nodes[id]
                                    .access()
                                    .contains(&AccessMeta::new(AccessType::World, Access::Read))
                                    .then_some(*id)
                            })
                            .collect::<Vec<_>>();

                        next.retain(|id| !world_ids.contains(id));
                        let mut next_index = hierarchy.len();
                        hierarchy.push(next);

                        for id in world_ids {
                            if self.nodes.index(&id).unwrap()
                                < self.nodes.index(hierarchy[next_index].first().unwrap()).unwrap()
                            {
                                hierarchy.insert(next_index, vec![id]);
                                next_index += 1;
                            }
                        }
                    }
                }
            }
        }

        self.hierarchy = hierarchy;
    }
}
