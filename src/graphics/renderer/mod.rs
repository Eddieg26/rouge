use self::context::RenderContext;

use super::{
    core::gpu::GpuInstance,
    resources::{
        buffer::{Buffer, BufferInfo},
        texture::{Texture, Texture2d, TextureInfo},
        BufferId, GraphicsResources, ResourceId, TextureId,
    },
    scene::GraphicsScene,
};
use std::{collections::HashMap, rc::Rc};

pub type NodeId = ResourceId;

pub mod context;
pub mod pass;

pub trait RenderGraphNode: 'static {
    fn id(&self) -> &NodeId;
    fn dependencies(&self) -> &Vec<NodeId>;
    fn dependencies_mut(&mut self) -> &mut Vec<NodeId>;
    fn add_dependency(&mut self, id: NodeId) {
        self.dependencies_mut().push(id);
    }
    fn execute(&self, ctx: &RenderContext, encoder: &mut wgpu::CommandEncoder);
}

pub struct RenderGraph {
    gpu: Rc<GpuInstance>,
    nodes: Vec<Box<dyn RenderGraphNode>>,
    texture_infos: HashMap<TextureId, TextureInfo>,
    buffer_infos: HashMap<TextureId, BufferInfo>,
    textures: HashMap<TextureId, Box<dyn Texture>>,
    buffers: HashMap<BufferId, Buffer>,
    is_dirty: bool,
}

impl RenderGraph {
    pub fn new(gpu: Rc<GpuInstance>) -> RenderGraph {
        RenderGraph {
            gpu,
            nodes: Vec::new(),
            texture_infos: HashMap::new(),
            buffer_infos: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
            is_dirty: false,
        }
    }

    pub(crate) fn build(&mut self) {
        self.sort();

        let gpu = &self.gpu;

        for (id, info) in &self.texture_infos {
            let texture: Box<dyn Texture> = match info.dimension {
                wgpu::TextureDimension::D1 => todo!(),
                wgpu::TextureDimension::D2 => {
                    Box::new(Texture2d::from_info(gpu.device(), gpu.queue(), info))
                }
                wgpu::TextureDimension::D3 => todo!(),
            };

            self.textures.insert(*id, texture);
        }

        for (id, info) in &self.buffer_infos {
            let buffer = Buffer::from_info(gpu.device(), info);

            self.buffers.insert(*id, buffer);
        }
    }

    pub fn gpu(&self) -> &GpuInstance {
        &self.gpu
    }

    pub fn create_texture(&mut self, id: impl Into<TextureId>, info: TextureInfo) -> TextureId {
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

    fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Graph Encoder"),
            })
    }

    pub fn execute(
        &mut self,
        scene: &GraphicsScene,
        resources: &GraphicsResources,
    ) -> Result<(), String> {
        if self.is_dirty {
            self.sort();
            self.is_dirty = false;
        }

        let surface = self
            .gpu
            .surface()
            .get_current_texture()
            .map_err(|e| e.to_string())?;

        let view = surface
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let ctx = RenderContext::new(
            &self.gpu,
            scene,
            resources,
            &self.textures,
            &self.buffers,
            &view,
        );
        let mut encoder = self.create_encoder();

        for node in &self.nodes {
            node.execute(&ctx, &mut encoder);
        }

        self.gpu.queue().submit(std::iter::once(encoder.finish()));

        surface.present();

        Ok(())
    }
}
