use rouge_ecs::macros::Resource;

#[derive(Resource)]
pub struct RenderDevice(wgpu::Device);

impl RenderDevice {
    pub fn new(device: wgpu::Device) -> Self {
        Self(device)
    }
}

impl std::ops::Deref for RenderDevice {
    type Target = wgpu::Device;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource)]
pub struct RenderQueue(wgpu::Queue);

impl RenderQueue {
    pub fn new(queue: wgpu::Queue) -> Self {
        Self(queue)
    }
}

impl std::ops::Deref for RenderQueue {
    type Target = wgpu::Queue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
