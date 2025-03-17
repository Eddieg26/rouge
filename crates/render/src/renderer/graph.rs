use super::{
    BufferDesc, RenderGraphBuffer, RenderGraphResource, RenderGraphResources, RenderGraphTexture,
    TextureDesc, context::RenderContext,
};
use crate::{
    device::RenderDevice,
    resources::{Id, extract::RenderAssets, texture::RenderTarget},
    surface::RenderSurface,
};
use ecs::{IndexMap, Resource, world::World};
use std::{any::TypeId, collections::HashMap};
use wgpu::{BufferDescriptor, TextureDescriptor};

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

    fn run(&self, ctx: &mut RenderContext);
}
downcast_rs::impl_downcast!(RenderGraphNode);

pub struct RenderGraphBuilder {
    resources: Vec<RenderGraphResource>,
    textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
    buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
    nodes: IndexMap<NodeId, Box<dyn RenderGraphNode>>,
    edges: Vec<NodeEdge>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
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

    pub fn add_texture(&mut self, name: &str, desc: TextureDesc) -> Id<RenderGraphTexture> {
        let id = name.into();
        self.resources
            .push(RenderGraphResource::Texture { id, desc });

        id
    }

    pub fn add_buffer(&mut self, name: &str, desc: BufferDesc) -> Id<RenderGraphBuffer> {
        let id = name.into();
        self.resources
            .push(RenderGraphResource::Buffer { id, desc });

        id
    }

    pub fn import_texture(
        &mut self,
        id: impl Into<Id<RenderGraphTexture>>,
        texture: wgpu::TextureView,
    ) -> Id<RenderGraphTexture> {
        let id = id.into();
        self.textures.insert(
            id,
            RenderGraphTexture {
                view: texture,
                desc: None,
            },
        );

        id
    }

    pub fn import_buffer(
        &mut self,
        id: impl Into<Id<RenderGraphBuffer>>,
        buffer: wgpu::Buffer,
    ) -> Id<RenderGraphBuffer> {
        let id = id.into();
        self.buffers
            .insert(id, RenderGraphBuffer { buffer, desc: None });

        id
    }

    pub fn build(
        mut self,
        device: &RenderDevice,
        width: u32,
        height: u32,
    ) -> Result<RenderGraph, RenderGraphBuildError> {
        let order = self.build_order()?;

        for resource in self.resources {
            match resource {
                RenderGraphResource::Texture { id, desc } => {
                    let texture = device.create_texture(&TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 0,
                        dimension: wgpu::TextureDimension::D2,
                        format: desc.format,
                        usage: desc.usage,
                        view_formats: &[desc.format],
                    });
                    self.textures.insert(
                        id,
                        RenderGraphTexture {
                            view: texture.create_view(&Default::default()),
                            desc: Some(desc),
                        },
                    );
                }
                RenderGraphResource::Buffer { id, desc } => {
                    let buffer = device.create_buffer(&BufferDescriptor {
                        label: None,
                        size: desc.size,
                        usage: desc.usage,
                        mapped_at_creation: false,
                    });
                    self.buffers.insert(
                        id,
                        RenderGraphBuffer {
                            buffer,
                            desc: Some(desc),
                        },
                    );
                }
            }
        }

        let resources = RenderGraphResources::new(self.textures, self.buffers);

        Ok(RenderGraph::new(
            resources,
            self.nodes.into_values().collect(),
            order,
        ))
    }

    fn build_order(&self) -> Result<Vec<Vec<usize>>, RenderGraphBuildError> {
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
                return Err(RenderGraphBuildError::CyclicDependency);
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
pub enum RenderGraphBuildError {
    CyclicDependency,
}

impl std::fmt::Display for RenderGraphBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CyclicDependency => {
                write!(f, "RenderGraphError: Cyclic dependency detected")
            }
        }
    }
}

impl std::error::Error for RenderGraphBuildError {}

pub struct RenderGraph {
    resources: RenderGraphResources,
    nodes: Vec<Box<dyn RenderGraphNode>>,
    order: Vec<Vec<usize>>,
}

impl RenderGraph {
    fn new(
        resources: RenderGraphResources,
        nodes: Vec<Box<dyn RenderGraphNode>>,
        order: Vec<Vec<usize>>,
    ) -> Self {
        Self {
            resources,
            nodes,
            order,
        }
    }

    pub fn resources(&self) -> &RenderGraphResources {
        &self.resources
    }

    pub fn nodes(&self) -> &[Box<dyn RenderGraphNode>] {
        &self.nodes
    }

    pub fn run(&self, world: &World) {
        let device = world.resource::<RenderDevice>();
        let targets = world.resource::<RenderAssets<RenderTarget>>();
        let Some(target) = targets.get(&RenderSurface::ID) else {
            return;
        };

        for group in &self.order {
            let mut buffers = vec![];

            for &index in group {
                let mut context =
                    RenderContext::new(world, device, target, targets, &self.resources);
                self.nodes[index].run(&mut context);

                buffers.extend(context.finish());
            }

            if !buffers.is_empty() {
                device.queue.submit(buffers.drain(..));
                device.queue.on_submitted_work_done(|| {});
            }
        }
    }
}

impl Resource for RenderGraph {}
