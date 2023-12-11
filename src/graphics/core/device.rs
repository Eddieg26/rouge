use crate::ecs::Resource;

pub struct RenderDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl RenderDevice {
    pub fn new(adapter: &wgpu::Adapter) -> RenderDevice {
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Render Device"),
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        RenderDevice { device, queue }
    }

    pub fn inner(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

impl Resource for RenderDevice {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
