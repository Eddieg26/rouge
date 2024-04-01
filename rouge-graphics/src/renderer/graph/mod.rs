use self::{
    context::RenderContext,
    nodes::{GraphNode, NodeId, RenderPhase},
    resources::{GraphResources, TextureDesc},
};
use crate::{
    core::{
        device::RenderDevice,
        surface::{RenderSurface, RenderSurfaceTexture},
    },
    resources::buffer::BaseBuffer,
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

    pub fn node_mut<N: GraphNode>(&mut self, id: impl Into<NodeId>) -> Option<&mut N> {
        let id = id.into();
        self.nodes
            .get_mut(&id)
            .map(|n| n.downcast_mut::<N>().unwrap())
    }

    pub fn texture(&self, id: impl Into<ResourceId>) -> Option<&wgpu::TextureView> {
        self.resources.texture(id)
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

        assert!(
            node.phase() != RenderPhase::Present
                || self
                    .nodes
                    .iter()
                    .all(|(_, n)| n.phase() != RenderPhase::Present),
            "Only one present node is allowed"
        );

        self.nodes.insert(id, Box::new(node));
        self
    }

    pub fn import_texture(
        &mut self,
        id: impl Into<ResourceId>,
        texture: wgpu::TextureView,
    ) -> &mut Self {
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

    pub fn remove_texture(&mut self, id: impl Into<ResourceId>) -> Option<wgpu::TextureView> {
        self.resources.remove_texture(id)
    }

    pub fn build(&mut self, device: &RenderDevice, surface: &RenderSurface) {
        let size = surface.size();
        self.resources.build(device, size.width, size.height);
        self.build_hierarchy();
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.resources.build(device, width, height);
    }

    pub fn execute(&mut self, world: &World) {
        let ctx = RenderContext::new(world, &self.resources, world.resource::<RenderDevice>());

        for ids in &self.hierarchy {
            for id in ids {
                let node = &mut self.nodes[id];
                node.prepare(ctx.clone());
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
                                < self
                                    .nodes
                                    .index(hierarchy[next_index].first().unwrap())
                                    .unwrap()
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

fn insert_surface_depth_texture(surface: &RenderSurface, graph: &mut RenderGraph) {
    let depth_desc = TextureDesc {
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        ..Default::default()
    };

    let id = ResourceId::from(GraphResources::SURFACE_DEPTH_ID);
    graph.create_texture(id, depth_desc);
}

fn insert_surface_view(
    graph: &mut RenderGraph,
    surface_texture: &mut RenderSurfaceTexture,
    surface: &RenderSurface,
) {
    let texture = surface.inner().get_current_texture().unwrap();
    let view = texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    graph
        .resources
        .import_texture(GraphResources::SURFACE_ID, view);

    surface_texture.set(texture);
}

fn remove_surface_view(graph: &mut RenderGraph) {
    graph.resources.remove_texture(GraphResources::SURFACE_ID);
}
