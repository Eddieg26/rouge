use super::resources::GraphResources;
use crate::{core::RenderDevice, surface::target::RenderTarget};
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
    actions: Vec<RenderNodeAction>,
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
            actions: Vec::new(),
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

    pub fn submit(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.actions.push(RenderNodeAction::Submit(command_buffer));
    }

    pub fn flush(&mut self) {
        self.actions.push(RenderNodeAction::Flush);
    }

    pub fn finish(self) -> Vec<RenderNodeAction> {
        self.actions
    }
}
