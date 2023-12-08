use crate::graphics::{
    core::gpu::GpuInstance,
    resources::{buffer::Buffer, texture::Texture, BufferId, GraphicsResources, TextureId},
    state::RenderState,
};
use std::collections::HashMap;

pub struct RenderContext<'a> {
    gpu: &'a GpuInstance,
    state: &'a RenderState,
    resources: &'a GraphicsResources,
    textures: &'a HashMap<TextureId, Box<dyn Texture>>,
    buffers: &'a HashMap<BufferId, Buffer>,
    render_target: &'a wgpu::TextureView,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        gpu: &'a GpuInstance,
        state: &'a RenderState,
        resources: &'a GraphicsResources,
        textures: &'a HashMap<TextureId, Box<dyn Texture>>,
        buffers: &'a HashMap<BufferId, Buffer>,
        render_target: &'a wgpu::TextureView,
    ) -> RenderContext<'a> {
        RenderContext {
            gpu,
            state,
            resources,
            textures,
            buffers,
            render_target,
        }
    }

    pub fn gpu(&self) -> &GpuInstance {
        self.gpu
    }

    pub fn state(&self) -> &RenderState {
        self.state
    }

    pub fn resources(&self) -> &GraphicsResources {
        self.resources
    }

    pub fn texture<T: Texture>(&self, id: &TextureId) -> Option<&T> {
        let texture = self.textures.get(id)?;
        texture.as_any().downcast_ref::<T>()
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.textures.get(id).map(|t| t.as_ref())
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    pub fn render_target(&self) -> &wgpu::TextureView {
        self.render_target
    }
}

pub struct RenderUpdateContext<'a> {
    gpu: &'a GpuInstance,
    resources: &'a GraphicsResources,
    textures: &'a HashMap<TextureId, Box<dyn Texture>>,
    buffers: &'a HashMap<BufferId, Buffer>,
}

impl<'a> RenderUpdateContext<'a> {
    pub fn new(
        gpu: &'a GpuInstance,
        resources: &'a GraphicsResources,
        textures: &'a HashMap<TextureId, Box<dyn Texture>>,
        buffers: &'a HashMap<BufferId, Buffer>,
    ) -> RenderUpdateContext<'a> {
        RenderUpdateContext {
            gpu,
            resources,
            textures,
            buffers,
        }
    }

    pub fn gpu(&self) -> &GpuInstance {
        self.gpu
    }

    pub fn resources(&self) -> &GraphicsResources {
        self.resources
    }

    pub fn texture<T: Texture>(&self, id: &TextureId) -> Option<&T> {
        let texture = self.textures.get(id)?;
        texture.as_any().downcast_ref::<T>()
    }

    pub fn dyn_texture(&self, id: &TextureId) -> Option<&dyn Texture> {
        self.textures.get(id).map(|t| t.as_ref())
    }

    pub fn buffer(&self, id: &BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
    }
}
