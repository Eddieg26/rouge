use super::{
    context::{GraphResources, RenderContext, RenderUpdateContext},
    NodeId,
};
use crate::{
    ecs::{Resource, World},
    graphics::{
        core::{device::RenderDevice, surface::RenderSurface},
        resources::{buffer::Buffer, texture::Texture, BufferId, TextureId},
        state::RenderState,
    },
};

pub trait RenderGraphNode: 'static {
    fn id(&self) -> &NodeId;
    fn dependencies(&self) -> &Vec<NodeId>;
    fn dependencies_mut(&mut self) -> &mut Vec<NodeId>;
    fn add_dependency(&mut self, id: NodeId) {
        self.dependencies_mut().push(id);
    }

    fn update(&mut self, _ctx: &RenderUpdateContext) {}
    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::CommandEncoder);
}

pub struct RenderGraph {
    nodes: Vec<Box<dyn RenderGraphNode>>,
    resources: GraphResources,
    is_dirty: bool,
}

impl RenderGraph {
    pub fn new() -> RenderGraph {
        RenderGraph {
            nodes: Vec::new(),
            resources: GraphResources::new(),
            is_dirty: false,
        }
    }

    pub fn build(&mut self, device: &RenderDevice, width: u32, height: u32) {
        self.sort();

        self.resources.update(device, width, height)
    }

    pub fn create_texture(
        &mut self,
        id: impl Into<TextureId>,
        usages: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
        dimension: wgpu::TextureDimension,
    ) -> TextureId {
        self.resources.create_texture(id, usages, format, dimension)
    }

    pub fn create_buffer(
        &mut self,
        id: impl Into<TextureId>,
        usages: wgpu::BufferUsages,
        size: wgpu::BufferAddress,
    ) -> BufferId {
        self.resources.create_buffer(id, usages, size)
    }

    pub fn import_texture<T: Texture>(
        &mut self,
        id: impl Into<TextureId>,
        texture: T,
    ) -> TextureId {
        self.resources.import_texture(id, texture)
    }

    pub fn import_buffer(&mut self, id: impl Into<BufferId>, buffer: Buffer) -> BufferId {
        self.resources.import_buffer(id, buffer)
    }

    pub fn add_node<T: RenderGraphNode>(&mut self, node: T) {
        self.nodes.push(Box::new(node));
        self.is_dirty = true;
    }

    pub fn remove_node(&mut self, id: NodeId) {
        let mut node_id = 0;

        for (index, node) in self.nodes.iter().enumerate() {
            if *node.id() == id {
                node_id = index;
                break;
            }
        }

        self.nodes.remove(node_id);
        self.is_dirty = true;
    }

    pub fn texture<T: Texture>(&self, id: &TextureId) -> Option<&T> {
        self.resources.texture::<T>(id)
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.resources.dyn_texture(id)
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffer(id)
    }

    fn sort(&mut self) -> &Vec<Box<dyn RenderGraphNode>> {
        let mut nodes = vec![];

        while !self.nodes.is_empty() {
            let mut node_id = 0;

            for (index, node) in self.nodes.iter().enumerate() {
                if !self.has_dependents(node.id()) {
                    node_id = index;
                    break;
                }
            }

            if node_id == 0 {
                panic!("Circular dependency detected in render graph");
            }

            nodes.insert(0, self.nodes.remove(node_id));
        }

        self.nodes = nodes;

        &self.nodes
    }

    fn has_dependents(&self, node_id: &NodeId) -> bool {
        for node in &self.nodes {
            if node.dependencies().contains(node_id) {
                return true;
            }
        }

        false
    }

    fn create_encoder(&self, device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Graph Encoder"),
        })
    }

    pub fn update(&mut self, world: &World) {
        if self.is_dirty {
            self.sort();
            self.is_dirty = false;
        }
        let device = world.resource::<RenderDevice>();
        let ctx = RenderUpdateContext::new(&device, &self.resources);

        for node in &mut self.nodes {
            node.update(&ctx);
        }
    }

    pub fn execute(&self, world: &World) -> Result<(), String> {
        let state = world.state::<RenderState>();
        let device = world.resource::<RenderDevice>();
        let surface = world.resource::<RenderSurface>();

        let surface = surface.current_texture().map_err(|e| e.to_string())?;

        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let ctx = RenderContext::new(world, &device, &state, &self.resources, &view);
        let mut encoder = self.create_encoder(device.inner());

        for node in &self.nodes {
            node.execute(&ctx, &mut encoder);
        }

        device.queue().submit(std::iter::once(encoder.finish()));

        surface.present();

        Ok(())
    }
}

impl Resource for RenderGraph {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
