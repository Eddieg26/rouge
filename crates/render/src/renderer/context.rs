use super::RenderGraphResources;
use crate::{
    device::RenderDevice,
    resources::{
        extract::RenderAssets,
        texture::{GpuTexture, RenderTarget},
    },
};
use ecs::world::World;

pub struct RenderContext<'a> {
    world: &'a World,
    device: &'a RenderDevice,
    surface: &'a RenderTarget,
    targets: &'a RenderAssets<RenderTarget>,
    resources: &'a RenderGraphResources,
    textures: &'a RenderAssets<GpuTexture>,
    buffers: Vec<wgpu::CommandBuffer>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        world: &'a World,
        device: &'a RenderDevice,
        surface: &'a RenderTarget,
        targets: &'a RenderAssets<RenderTarget>,
        resources: &'a RenderGraphResources,
    ) -> Self {
        Self {
            world,
            device,
            surface,
            targets,
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

    pub fn surface(&self) -> &'a RenderTarget {
        self.surface
    }

    pub fn resources(&self) -> &'a RenderGraphResources {
        self.resources
    }

    pub fn textures(&self) -> &'a RenderAssets<GpuTexture> {
        self.textures
    }

    pub fn targets(&self) -> &'a RenderAssets<RenderTarget> {
        self.targets
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
