use std::sync::{Arc, Mutex};

use super::resources::GraphResources;
use crate::core::device::RenderDevice;
use rouge_ecs::{SystemArg, World};

#[derive(Clone)]
pub struct RenderContext<'a> {
    world: &'a World,
    resources: &'a GraphResources,
    device: &'a RenderDevice,
    buffers: Arc<Mutex<Vec<wgpu::CommandBuffer>>>,
}

impl<'a> RenderContext<'a> {
    pub fn new(world: &'a World, resources: &'a GraphResources, device: &'a RenderDevice) -> Self {
        Self {
            world,
            resources,
            device,
            buffers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn system_arg<A: SystemArg>(&self) -> A::Item<'a> {
        A::get(self.world)
    }

    pub fn resources(&self) -> &'a GraphResources {
        &self.resources
    }

    pub fn device(&self) -> &RenderDevice {
        self.device
    }

    pub fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(encoder.finish());
    }

    pub (crate) fn collect(&self) -> Vec<wgpu::CommandBuffer> {
        self.buffers.lock().unwrap().drain(..).collect()
    }
}
