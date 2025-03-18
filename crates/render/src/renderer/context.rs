use super::RenderGraphResources;
use crate::{
    device::RenderDevice,
    resources::{extract::RenderAssets, texture::GpuTexture},
};
use ecs::world::World;

pub struct RenderContext<'a> {
    world: &'a World,
    device: &'a RenderDevice,
    target: &'a wgpu::TextureView,
    depth: &'a wgpu::TextureView,
    resources: &'a RenderGraphResources,
    textures: &'a RenderAssets<GpuTexture>,
    buffers: Vec<wgpu::CommandBuffer>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        world: &'a World,
        device: &'a RenderDevice,
        target: &'a wgpu::TextureView,
        depth: &'a wgpu::TextureView,
        resources: &'a RenderGraphResources,
    ) -> Self {
        Self {
            world,
            device,
            target,
            depth,
            resources,
            textures: world.resource::<RenderAssets<GpuTexture>>(),
            buffers: Vec::new(),
        }
    }

    pub fn world(&self) -> &'a World {
        self.world
    }

    pub fn device(&self) -> &'a RenderDevice {
        self.device
    }

    pub fn target(&self) -> &'a wgpu::TextureView {
        self.target
    }

    pub fn depth(&self) -> &'a wgpu::TextureView {
        self.depth
    }

    pub fn resources(&self) -> &'a RenderGraphResources {
        self.resources
    }

    pub fn textures(&self) -> &'a RenderAssets<GpuTexture> {
        self.textures
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&Default::default())
    }

    pub fn submit(&mut self, buffer: wgpu::CommandBuffer) {
        self.buffers.push(buffer);
    }

    pub fn finish(self) -> Vec<wgpu::CommandBuffer> {
        self.buffers
    }
}
