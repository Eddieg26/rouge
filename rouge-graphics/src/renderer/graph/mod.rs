use self::{
    nodes::{GraphNode, NodeId},
    resources::{GraphResources, TextureDesc},
};
use crate::resources::{buffer::BaseBuffer, texture::GpuTexture};
use rouge_core::ResourceId;
use rouge_ecs::macros::Resource;
use std::collections::HashMap;

pub mod context;
pub mod nodes;
pub mod resources;

#[derive(Resource)]
pub struct RenderGraph {
    resources: GraphResources,
    nodes: HashMap<NodeId, Box<dyn GraphNode>>,
    hierarchy: Vec<Vec<NodeId>>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            resources: GraphResources::new(),
            nodes: HashMap::new(),
            hierarchy: Vec::new(),
        }
    }

    pub fn node<N: GraphNode>(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(&id).map(|n| n.downcast_ref::<N>().unwrap())
    }

    pub fn node_mut<N: GraphNode>(&mut self, id: NodeId) -> Option<&mut N> {
        self.nodes
            .get_mut(&id)
            .map(|n| n.downcast_mut::<N>().unwrap())
    }

    pub fn add_node<N: GraphNode>(&mut self, id: NodeId, node: N) -> &mut Self {
        assert!(
            !self.nodes.contains_key(&id),
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

    pub fn create_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) -> &mut Self {
        self.resources.create_texture(id, desc);
        self
    }

    pub fn build(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.resources.build(device, width, height);
        self.build_heirarchy();
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.resources.build(device, width, height);
    }

    fn build_heirarchy(&mut self) {
        let mut hierarchy = Vec::new();
        for (id, node) in &self.nodes {
            let mut parents = Vec::new();
            for (parent_id, parent_node) in &self.nodes {
                if id != parent_id
                    && parent_node
                        .writes()
                        .iter()
                        .any(|r| node.reads().contains(r))
                {
                    parents.push(*parent_id);
                }
            }
            hierarchy.push(parents);
        }
        self.hierarchy = hierarchy;
    }
}
