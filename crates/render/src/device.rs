use ecs::{Resource, world::action::WorldAction};
use std::sync::Arc;
use wgpu::{Adapter, Device, Queue, RequestDeviceError};

pub struct RenderDevice {
    device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl RenderDevice {
    pub async fn new(adapter: &Adapter) -> Result<Self, RequestDeviceError> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }
}

impl std::ops::Deref for RenderDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Resource for RenderDevice {}

pub struct DeviceCreated;

impl WorldAction for DeviceCreated {
    fn execute(self, _: &mut ecs::world::World) -> Option<()> {
        todo!()
    }
}
