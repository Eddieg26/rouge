use wgpu::util::DeviceExt;

pub struct BufferInfo {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub mapped_at_creation: bool,
}

impl BufferInfo {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> BufferInfo {
        BufferInfo {
            size,
            usage,
            mapped_at_creation: false,
        }
    }

    pub fn new_type<T: bytemuck::Pod + bytemuck::Zeroable>(
        usage: wgpu::BufferUsages,
    ) -> BufferInfo {
        let size = std::mem::size_of::<T>() as u64;

        BufferInfo::new(size, usage)
    }

    pub fn mapped_at_creation(mut self, mapped_at_creation: bool) -> BufferInfo {
        self.mapped_at_creation = mapped_at_creation;

        self
    }
}

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

    pub fn from_info(device: &wgpu::Device, info: &BufferInfo) -> Buffer {
        let inner = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: info.size,
            usage: info.usage,
            mapped_at_creation: info.mapped_at_creation,
        });

        Buffer { inner }
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
