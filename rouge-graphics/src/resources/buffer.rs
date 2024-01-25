use crate::core::{BaseVertex, ResourceId};
use rouge_ecs::{macros::Resource, storage::sparse::SparseMap, world::resource::Resource};
use wgpu::util::DeviceExt;

pub trait BaseBuffer: Send + Sync + 'static {
    fn inner(&self) -> &wgpu::Buffer;
    fn len(&self) -> usize;
    fn usages(&self) -> wgpu::BufferUsages;
}

pub struct UniformBuffer {
    buffer: wgpu::Buffer,
    size: wgpu::BufferAddress,
    usages: wgpu::BufferUsages,
    len: usize,
}

impl UniformBuffer {
    pub fn new<T: bytemuck::Pod>(
        device: &wgpu::Device,
        data: &[T],
        usages: wgpu::BufferUsages,
    ) -> Self {
        let size = std::mem::size_of::<T>() as wgpu::BufferAddress;
        let len = data.len();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: usages | wgpu::BufferUsages::UNIFORM,
        });

        Self {
            buffer,
            size,
            usages: usages | wgpu::BufferUsages::UNIFORM,
            len,
        }
    }

    pub fn update<T: bytemuck::Pod>(&mut self, device: &wgpu::Device, data: &[T]) {
        assert_eq!(self.size, std::mem::size_of::<T>() as wgpu::BufferAddress);
        assert_eq!(self.len, data.len());

        self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: self.usages,
        });
    }
}

impl BaseBuffer for UniformBuffer {
    fn inner(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn len(&self) -> usize {
        self.len
    }

    fn usages(&self) -> wgpu::BufferUsages {
        self.usages
    }
}

pub struct VertexBuffer {
    buffer: wgpu::Buffer,
    usages: wgpu::BufferUsages,
    len: usize,
}

impl VertexBuffer {
    pub fn new<V: BaseVertex>(
        device: &wgpu::Device,
        data: &[V],
        usages: wgpu::BufferUsages,
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: usages | wgpu::BufferUsages::VERTEX,
        });

        Self {
            buffer,
            usages: usages | wgpu::BufferUsages::VERTEX,
            len: data.len(),
        }
    }

    pub fn update<V: BaseVertex>(&mut self, device: &wgpu::Device, data: &[V]) {
        assert_eq!(self.len, data.len());

        self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: self.usages,
        });
    }
}

impl BaseBuffer for VertexBuffer {
    fn inner(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn len(&self) -> usize {
        self.len
    }

    fn usages(&self) -> wgpu::BufferUsages {
        self.usages
    }
}

pub struct IndexBuffer {
    buffer: wgpu::Buffer,
    usages: wgpu::BufferUsages,
    len: usize,
}

impl IndexBuffer {
    pub fn new(device: &wgpu::Device, data: &[u32], usages: wgpu::BufferUsages) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: usages | wgpu::BufferUsages::INDEX,
        });

        Self {
            buffer,
            usages: usages | wgpu::BufferUsages::INDEX,
            len: data.len(),
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, data: &[u32]) {
        assert_eq!(self.len, data.len());

        self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: self.usages,
        });
    }
}

impl BaseBuffer for IndexBuffer {
    fn inner(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn len(&self) -> usize {
        self.len
    }

    fn usages(&self) -> wgpu::BufferUsages {
        self.usages
    }
}

pub struct StorageBuffer {
    buffer: wgpu::Buffer,
    size: wgpu::BufferAddress,
    usages: wgpu::BufferUsages,
    len: usize,
}

impl StorageBuffer {
    pub fn new<T: bytemuck::Pod>(
        device: &wgpu::Device,
        data: &[T],
        usages: wgpu::BufferUsages,
    ) -> Self {
        let size = std::mem::size_of::<T>() as wgpu::BufferAddress;
        let len = data.len();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: usages | wgpu::BufferUsages::STORAGE,
        });

        Self {
            buffer,
            size,
            usages: usages | wgpu::BufferUsages::STORAGE,
            len,
        }
    }

    pub fn update<T: bytemuck::Pod>(&mut self, device: &wgpu::Device, data: &[T]) {
        assert_eq!(self.size, std::mem::size_of::<T>() as wgpu::BufferAddress);
        assert_eq!(self.len, data.len());

        self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(data),
            usage: self.usages,
        });
    }
}

impl BaseBuffer for StorageBuffer {
    fn inner(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn len(&self) -> usize {
        self.len
    }

    fn usages(&self) -> wgpu::BufferUsages {
        self.usages
    }
}

#[derive(Resource)]
pub struct Buffers<T: BaseBuffer> {
    buffers: SparseMap<ResourceId, T>,
}

impl<T: BaseBuffer> Buffers<T> {
    pub fn new() -> Self {
        Self {
            buffers: SparseMap::new(),
        }
    }

    pub fn insert(&mut self, id: impl Into<ResourceId>, buffer: T) {
        self.buffers.insert(id.into(), buffer);
    }

    pub fn get(&self, id: impl Into<ResourceId>) -> Option<&T> {
        self.buffers.get(&id.into())
    }

    pub fn get_mut(&mut self, id: impl Into<ResourceId>) -> Option<&mut T> {
        self.buffers.get_mut(&id.into())
    }

    pub fn remove(&mut self, id: impl Into<ResourceId>) -> Option<T> {
        self.buffers.remove(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ResourceId, &T)> {
        self.buffers.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ResourceId, &mut T)> {
        self.buffers.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}
