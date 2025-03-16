use super::resources::{GraphResources, RenderGraphBuffer, RenderGraphTexture};
use crate::{
    core::RenderDevice,
    extract::RenderAssets,
    resource::{
        texture::{render::RenderTarget, GpuTexture},
        Id,
    },
};
use ecs::{core::resource::Resource, world::World};

pub enum RenderNodeAction {
    Submit(wgpu::CommandBuffer),
    Flush,
}

pub struct RenderContext<'a> {
    world: &'a World,
    device: &'a RenderDevice,
    resources: &'a GraphResources,
    target: &'a RenderTarget,
    targets: &'a RenderAssets<RenderTarget>,
    textures: &'a RenderAssets<GpuTexture>,
    buffers: Vec<wgpu::CommandBuffer>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        world: &'a World,
        device: &'a RenderDevice,
        resources: &'a GraphResources,
        target: &'a RenderTarget,
    ) -> Self {
        Self {
            world,
            device,
            resources,
            target,
            targets: world.resource::<RenderAssets<RenderTarget>>(),
            textures: world.resource::<RenderAssets<GpuTexture>>(),
            buffers: Vec::new(),
        }
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn resources(&self) -> &GraphResources {
        self.resources
    }

    pub fn target(&self) -> &RenderTarget {
        self.target
    }

    pub fn get_render_target(&self, id: impl Into<Id<RenderTarget>>) -> Option<&RenderTarget> {
        self.targets.get(&id.into())
    }

    pub fn textures(&self) -> &RenderAssets<GpuTexture> {
        self.textures
    }

    pub fn texture(&self, id: &Id<GpuTexture>) -> Option<&GpuTexture> {
        self.textures.get(id)
    }

    pub fn graph_texture(&self, id: &Id<RenderGraphTexture>) -> Option<&RenderGraphTexture> {
        self.resources.texture(id)
    }

    pub fn graph_buffer(&self, id: &Id<RenderGraphBuffer>) -> Option<&RenderGraphBuffer> {
        self.resources.buffer(id)
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.world.non_send_resource::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.world.try_resource::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.world.try_non_send_resource::<R>()
    }

    pub fn encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    pub fn submit(&mut self, encoder: wgpu::CommandEncoder) {
        self.buffers.push(encoder.finish());
    }

    pub fn finish(self) -> Vec<wgpu::CommandBuffer> {
        self.buffers
    }
}
