use crate::{
    ecs::World,
    graphics::{
        core::gpu::GpuInstance,
        resources::{
            buffer::{Buffer, BufferInfo},
            texture::{Texture, Texture2d, TextureDesc},
            BufferId, GraphicsResources, TextureId,
        },
        state::RenderState,
    },
};
use std::{collections::HashMap, rc::Rc};

use super::{
    context::{RenderContext, RenderUpdateContext},
    NodeId,
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
    texture_infos: HashMap<TextureId, TextureDesc>,
    buffer_infos: HashMap<TextureId, BufferInfo>,
    textures: HashMap<TextureId, Box<dyn Texture>>,
    buffers: HashMap<BufferId, Buffer>,
    is_dirty: bool,
}

impl RenderGraph {
    pub fn new() -> RenderGraph {
        RenderGraph {
            nodes: Vec::new(),
            texture_infos: HashMap::new(),
            buffer_infos: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
            is_dirty: false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn build(&mut self, gpu: Rc<GpuInstance>) {
        self.sort();

        for (id, info) in &self.texture_infos {
            let texture: Box<dyn Texture> = match info.dimension {
                wgpu::TextureDimension::D1 => todo!(),
                wgpu::TextureDimension::D2 => Box::new(Texture2d::from_desc(gpu.device(), info)),
                wgpu::TextureDimension::D3 => todo!(),
            };

            self.textures.insert(*id, texture);
        }

        for (id, info) in &self.buffer_infos {
            let buffer = Buffer::from_info(gpu.device(), info);

            self.buffers.insert(*id, buffer);
        }
    }

    pub fn create_texture(&mut self, id: impl Into<TextureId>, info: TextureDesc) -> TextureId {
        let id = id.into();
        self.texture_infos.insert(id, info);

        id
    }

    pub fn create_buffer(&mut self, id: impl Into<TextureId>, info: BufferInfo) -> BufferId {
        let id = id.into();
        self.buffer_infos.insert(id, info);

        id
    }

    pub fn import_texture<T: Texture>(
        &mut self,
        id: impl Into<TextureId>,
        texture: T,
    ) -> TextureId {
        let id = id.into();
        self.textures.insert(id, Box::new(texture));

        id
    }

    pub fn import_buffer(&mut self, id: impl Into<BufferId>, buffer: Buffer) -> BufferId {
        let id = id.into();
        self.buffers.insert(id, buffer);

        id
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
        self.textures
            .get(id)
            .map(|t| t.as_any().downcast_ref::<T>().unwrap())
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.textures.get(id).map(|t| t.as_ref())
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
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

    pub fn update(&mut self, gpu: &GpuInstance, resources: &GraphicsResources) {
        if self.is_dirty {
            self.sort();
            self.is_dirty = false;
        }

        let ctx = RenderUpdateContext::new(gpu, resources, &self.textures, &self.buffers);

        for node in &mut self.nodes {
            node.update(&ctx);
        }
    }

    pub fn execute(&self, world: &World) -> Result<(), String> {
        let graphics = world.resource::<GraphicsResources>();
        let state = world.state::<RenderState>();
        let gpu = graphics.gpu();

        let surface = gpu
            .surface()
            .get_current_texture()
            .map_err(|e| e.to_string())?;

        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let ctx = RenderContext::new(gpu, &state, &graphics, &self.textures, &self.buffers, &view);
        let mut encoder = self.create_encoder(gpu.device());

        for node in &self.nodes {
            node.execute(&ctx, &mut encoder);
        }

        gpu.queue().submit(std::iter::once(encoder.finish()));

        surface.present();

        Ok(())
    }
}
