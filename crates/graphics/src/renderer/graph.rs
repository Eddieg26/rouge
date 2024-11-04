use super::{
    context::RenderContext,
    resources::{
        BufferDesc, GraphResource, GraphResourceId, GraphResources, Id, RenderGraphBuffer,
        RenderGraphTexture, TextureDesc,
    },
};
use crate::{core::RenderDevice, surface::RenderSurface};
use ecs::core::IndexMap;
use std::any::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(TypeId);

impl NodeId {
    pub fn new<T: RenderGraphNode>() -> Self {
        Self(TypeId::of::<T>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeSlot {
    Read {
        node: NodeId,
        resource: GraphResourceId,
    },
    Write {
        node: NodeId,
        resource: GraphResourceId,
    },
}

impl EdgeSlot {
    pub fn read<T: RenderGraphNode, R: GraphResource>(resource: &str) -> Self {
        Self::Read {
            node: NodeId::new::<T>(),
            resource: R::id(resource),
        }
    }

    pub fn write<T: RenderGraphNode, R: GraphResource>(resource: &str) -> Self {
        Self::Write {
            node: NodeId::new::<T>(),
            resource: R::id(resource),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeEdge {
    pub from: EdgeSlot,
    pub to: EdgeSlot,
}

pub trait RenderGraphNode: downcast_rs::Downcast + Sync + 'static {
    fn name(&self) -> &str;

    fn run(&mut self, ctx: &RenderContext);
}
downcast_rs::impl_downcast!(RenderGraphNode);

pub struct RenderGraphBuilder {
    resources: GraphResources,
    nodes: IndexMap<TypeId, Box<dyn RenderGraphNode>>,
    edges: Vec<NodeEdge>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            resources: GraphResources::new(),
            nodes: IndexMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn node<T: RenderGraphNode>(&self) -> Option<&T> {
        self.nodes
            .get(&TypeId::of::<T>())
            .and_then(|node| node.downcast_ref::<T>())
    }

    pub fn node_mut<T: RenderGraphNode>(&mut self) -> Option<&mut T> {
        self.nodes
            .get_mut(&TypeId::of::<T>())
            .and_then(|node| node.downcast_mut::<T>())
    }

    pub fn nodes(&self) -> impl Iterator<Item = &dyn RenderGraphNode> {
        self.nodes.values().map(|node| &**node)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn add_node<T: RenderGraphNode>(&mut self, node: T) {
        self.nodes.insert(TypeId::of::<T>(), Box::new(node));
    }

    pub fn create_texture(&mut self, name: &str, desc: TextureDesc) -> Id<RenderGraphTexture> {
        self.resources.create_texture(name, desc)
    }

    pub fn create_buffer(&mut self, name: &str, desc: BufferDesc) -> Id<RenderGraphBuffer> {
        self.resources.create_buffer(name, desc)
    }

    pub fn import_texture(
        &mut self,
        name: &str,
        texture: RenderGraphTexture,
    ) -> Id<RenderGraphTexture> {
        self.resources.import_texture(name, texture)
    }

    pub fn import_buffer(
        &mut self,
        name: &str,
        buffer: RenderGraphBuffer,
    ) -> Id<RenderGraphBuffer> {
        self.resources.import_buffer(name, buffer)
    }

    pub fn add_edge(&mut self, from: impl Into<EdgeSlot>, to: impl Into<EdgeSlot>) {
        self.edges.push(NodeEdge {
            from: from.into(),
            to: to.into(),
        });
    }

    pub fn build(mut self, device: &RenderDevice, surface: &RenderSurface) -> RenderGraph {
        todo!()
    }
}

pub struct RenderGraph {
    resources: GraphResources,
    nodes: IndexMap<TypeId, Box<dyn RenderGraphNode>>,
    order: Vec<Vec<usize>>,
}

impl RenderGraph {
    fn new(
        resources: GraphResources,
        nodes: IndexMap<TypeId, Box<dyn RenderGraphNode>>,
        order: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            resources,
            nodes,
            order,
        }
    }
}
