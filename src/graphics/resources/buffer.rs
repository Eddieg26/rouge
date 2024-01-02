use super::shader::layout::ShaderVariable;
use crate::ecs::{resource::ResourceId, Resource};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub struct BufferDesc {
    pub usage: wgpu::BufferUsages,
    pub size: u64,
    pub mapped_at_creation: bool,
}

impl Default for BufferDesc {
    fn default() -> Self {
        BufferDesc {
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            size: 0,
            mapped_at_creation: false,
        }
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

    pub fn from_size(device: &wgpu::Device, usage: wgpu::BufferUsages, size: u64) -> Buffer {
        let inner = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
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

pub struct Buffers {
    buffers: HashMap<ResourceId, Buffer>,
}

impl Buffers {
    pub fn new() -> Buffers {
        Buffers {
            buffers: HashMap::new(),
        }
    }

    pub fn get(&self, id: &ResourceId) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    pub fn get_mut(&mut self, id: &ResourceId) -> Option<&mut Buffer> {
        self.buffers.get_mut(id)
    }

    pub fn register_buffer(
        &mut self,
        device: &wgpu::Device,
        inputs: &[ShaderVariable],
    ) -> ResourceId {
        let id = inputs.into();

        let size = inputs.iter().map(|i| i.size()).sum::<usize>() as u64;
        let buffer = Buffer::from_size(
            device,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            size,
        );

        self.buffers.insert(id, buffer);

        id
    }

    pub fn create_buffer(
        &mut self,
        device: &wgpu::Device,
        id: ResourceId,
        usages: wgpu::BufferUsages,
        bytes: &[u8],
    ) -> ResourceId {
        let buffer = Buffer::from_bytes(device, usages, bytes);

        self.buffers.insert(id, buffer);

        id
    }

    pub fn update<T: bytemuck::Pod + bytemuck::Zeroable>(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        id: &ResourceId,
        data: &T,
    ) {
        if let Some(buffer) = self.get(id) {
            buffer.udpate(device, encoder, data);
        }
    }
}

impl Resource for Buffers {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
