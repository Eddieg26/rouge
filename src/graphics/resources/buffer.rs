use wgpu::util::DeviceExt;

pub struct Buffer {
    inner: wgpu::Buffer,
}

impl Buffer {
    pub fn from_bytes(device: &wgpu::Device, usage: wgpu::BufferUsages, bytes: &[u8]) -> Buffer {
        let inner = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytes,
            label: None,
            usage,
        });

        Buffer { inner }
    }

    pub fn from_data<T: bytemuck::Pod + bytemuck::Zeroable>(
        device: &wgpu::Device,
        usage: wgpu::BufferUsages,
        data: &T,
    ) -> Buffer {
        let contents = bytemuck::bytes_of(data);

        Buffer::from_bytes(device, usage, contents)
    }

    pub fn inner(&self) -> &wgpu::Buffer {
        &self.inner
    }

    pub fn udpate<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &T,
    ) {
        let usage = self.inner.usage()
            & (wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::STORAGE)
            | wgpu::BufferUsages::COPY_SRC;

        let buffer = Buffer::from_data::<T>(device, usage, data);

        encoder.copy_buffer_to_buffer(
            buffer.inner(),
            0,
            self.inner(),
            0,
            std::mem::size_of::<T>() as u64,
        );
    }
}
