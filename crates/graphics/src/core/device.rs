use ecs::core::resource::Resource;
use std::sync::Arc;

pub struct RenderInstance(wgpu::Instance);

impl RenderInstance {
    pub fn create() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        Self(instance)
    }
}

impl std::ops::Deref for RenderInstance {
    type Target = wgpu::Instance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct RenderDevice {
    device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl RenderDevice {
    pub async fn create(adapter: &wgpu::Adapter) -> Result<Self, wgpu::RequestDeviceError> {
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    pub async fn dummy() -> Result<Self, wgpu::RequestDeviceError> {
        let instance = RenderInstance::create();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }
}

impl std::ops::Deref for RenderDevice {
    type Target = Arc<wgpu::Device>;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Resource for RenderDevice {}
