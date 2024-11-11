use super::{
    context::{RenderContext, RenderNodeAction},
    resources::{BufferDesc, GraphResources, RenderGraphBuffer, RenderGraphTexture, TextureDesc},
};
use crate::{
    core::{ExtractError, RenderAssets, RenderDevice, RenderResourceExtractor},
    resources::Id,
    surface::{target::RenderTarget, RenderSurface},
};
use ecs::{
    core::{resource::Resource, IndexMap},
    system::{unlifetime::ReadRes, StaticArg},
    world::{access::Removed, World},
};
use spatial::size::Size;
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(TypeId);

impl NodeId {
    pub fn new<T: RenderGraphNode>() -> Self {
        Self(TypeId::of::<T>())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeEdge {
    pub from: NodeId,
    pub to: NodeId,
}

impl From<(NodeId, NodeId)> for NodeEdge {
    fn from((from, to): (NodeId, NodeId)) -> Self {
        Self { from, to }
    }
}

pub trait RenderGraphNode: downcast_rs::Downcast + Send + Sync + 'static {
    fn name(&self) -> &str;

    fn run(&mut self, ctx: &mut RenderContext);
}
downcast_rs::impl_downcast!(RenderGraphNode);

pub struct RenderGraphBuilder {
    surface: Id<RenderTarget>,
    resources: GraphResources,
    nodes: IndexMap<NodeId, Box<dyn RenderGraphNode>>,
    edges: Vec<NodeEdge>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            surface: RenderSurface::ID,
            resources: GraphResources::new(),
            nodes: IndexMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn node<T: RenderGraphNode>(&self) -> Option<&T> {
        self.nodes
            .get(&NodeId::new::<T>())
            .and_then(|node| node.downcast_ref::<T>())
    }

    pub fn node_mut<T: RenderGraphNode>(&mut self) -> Option<&mut T> {
        self.nodes
            .get_mut(&NodeId::new::<T>())
            .and_then(|node| node.downcast_mut::<T>())
    }

    pub fn nodes(&self) -> impl Iterator<Item = &dyn RenderGraphNode> {
        self.nodes.values().map(|node| &**node)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn add_node<T: RenderGraphNode>(&mut self, node: T) {
        self.nodes.insert(NodeId::new::<T>(), Box::new(node));
    }

    pub fn add_edge<From: RenderGraphNode, To: RenderGraphNode>(&mut self) {
        let from = NodeId::new::<From>();
        let to = NodeId::new::<To>();
        self.edges.push(NodeEdge { from, to });
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

    pub fn set_surface(&mut self, surface: Id<RenderTarget>) {
        self.surface = surface;
    }

    pub fn build(
        mut self,
        device: &RenderDevice,
        size: Size,
    ) -> Result<RenderGraph, RenderGraphError> {
        self.resources.build(device);
        self.resources.resize(device, size);

        let order = self.build_order()?;
        Ok(RenderGraph::new(
            self.surface,
            self.resources,
            self.nodes,
            order,
        ))
    }

    fn build_order(&self) -> Result<Vec<Vec<usize>>, RenderGraphError> {
        let mut order = Vec::new();
        let mut dependencies = self
            .nodes
            .keys()
            .map(|id| (id, Vec::new()))
            .collect::<HashMap<_, _>>();

        for edge in &self.edges {
            dependencies
                .entry(&edge.to)
                .or_insert_with(Vec::new)
                .push(&edge.from);
        }

        while !dependencies.is_empty() {
            let mut group = Vec::new();
            for (&node, deps) in dependencies.iter() {
                if deps.iter().all(|dep| !dependencies.contains_key(dep)) {
                    group.push(node);
                }
            }

            if group.is_empty() {
                return Err(RenderGraphError::CyclicDependency);
            }

            dependencies.retain(|&node, _| !group.contains(&node));
            order.push(group);
        }

        Ok(order
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|id| self.nodes.get_index_of(*id).unwrap())
                    .collect::<Vec<_>>()
            })
            .collect())
    }
}

impl Resource for RenderGraphBuilder {}

#[derive(Debug, Clone)]
pub enum RenderGraphError {
    CyclicDependency,
}

impl std::fmt::Display for RenderGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CyclicDependency => {
                write!(f, "RenderGraphError: Cyclic dependency detected")
            }
        }
    }
}

impl std::error::Error for RenderGraphError {}

pub struct RenderGraph {
    resources: GraphResources,
    nodes: IndexMap<NodeId, Box<dyn RenderGraphNode>>,
    order: Vec<Vec<usize>>,
    surface: Id<RenderTarget>,
}

impl RenderGraph {
    fn new(
        surface: Id<RenderTarget>,
        resources: GraphResources,
        nodes: IndexMap<NodeId, Box<dyn RenderGraphNode>>,
        order: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            surface,
            resources,
            nodes,
            order,
        }
    }

    pub fn resources(&self) -> &GraphResources {
        &self.resources
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

    pub fn remove_texture(&mut self, handle: Id<RenderGraphTexture>) {
        self.resources.remove_texture(handle);
    }

    pub fn remove_buffer(&mut self, handle: Id<RenderGraphBuffer>) {
        self.resources.remove_buffer(handle);
    }

    pub fn resize(&mut self, device: &RenderDevice, size: Size) {
        self.resources.resize(device, size);
    }

    pub fn run(&mut self, world: &World) {
        let device = world.resource::<RenderDevice>();
        let targets = world.resource::<RenderAssets<RenderTarget>>();
        let target = match targets.get(&self.surface) {
            Some(target) => target,
            None => return,
        };

        for group in &self.order {
            let mut actions = vec![];
            for node in group {
                let mut ctx = RenderContext::new(world, device, &self.resources, target);
                self.nodes[*node].run(&mut ctx);

                let buffers = ctx.finish();
                if !buffers.is_empty() {
                    actions.extend(buffers);
                    actions.push(RenderNodeAction::Flush);
                }
            }

            let mut buffers = vec![];
            for action in actions {
                match action {
                    RenderNodeAction::Submit(buffer) => buffers.push(buffer),
                    RenderNodeAction::Flush => {
                        if !buffers.is_empty() {
                            device.queue.submit(buffers.drain(..));
                            device.queue.on_submitted_work_done(|| {});
                        }
                    }
                }
            }
        }
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self {
            surface: RenderSurface::ID,
            resources: GraphResources::new(),
            nodes: Default::default(),
            order: Default::default(),
        }
    }
}

impl Resource for RenderGraph {}

impl RenderResourceExtractor for RenderGraph {
    type Resource = RenderGraph;
    type Arg = StaticArg<
        'static,
        (
            ReadRes<RenderDevice>,
            ReadRes<RenderAssets<RenderTarget>>,
            Removed<RenderGraphBuilder>,
        ),
    >;

    fn extract(arg: ecs::system::ArgItem<Self::Arg>) -> Result<Self::Resource, ExtractError> {
        let (device, targets, builder) = arg.into_inner();
        if let Some(builder) = builder.into_inner() {
            match builder.build(&device, targets.max_size()) {
                Ok(graph) => Ok(graph),
                Err(error) => Err(ExtractError::from_error(error)),
            }
        } else {
            Ok(Default::default())
        }
    }

    fn default() -> Option<Self::Resource> {
        Some(Default::default())
    }
}
