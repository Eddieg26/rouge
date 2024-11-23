use super::{BatchedUniformBuffer, Buffer, BufferArray, BufferArrayIndex, BufferData};
use crate::{core::RenderDevice, resources::Label};
use wgpu::{BindingResource, BufferUsages};

pub enum RenderBufferArray<B: BufferData> {
    Uniform(BatchedUniformBuffer<B>),
    Storage(BufferArray<B>),
}

impl<B: BufferData> RenderBufferArray<B> {
    pub fn new(device: &RenderDevice, usage: BufferUsages) -> Self {
        match device.limits().max_storage_buffers_per_shader_stage {
            0 => Self::Uniform(BatchedUniformBuffer::new(device).with_usage(usage)),
            _ => Self::Storage(BufferArray::new(usage)),
        }
    }

    pub fn label(&self) -> &Label {
        match self {
            Self::Uniform(buffer) => buffer.label(),
            Self::Storage(buffer) => buffer.label(),
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        match self {
            Self::Uniform(buffer) => buffer.buffer(),
            Self::Storage(buffer) => buffer.buffer(),
        }
    }

    pub fn usage(&self) -> BufferUsages {
        match self {
            Self::Uniform(buffer) => buffer.usage(),
            Self::Storage(buffer) => buffer.usage(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Uniform(buffer) => buffer.is_empty(),
            Self::Storage(buffer) => buffer.is_empty(),
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            Self::Uniform(buffer) => buffer.binding(),
            Self::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn push(&mut self, value: B) -> BufferArrayIndex<B> {
        match self {
            Self::Uniform(buffer) => buffer.push(value),
            Self::Storage(buffer) => BufferArrayIndex::new(buffer.push(value) as u32, None),
        }
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match self {
            Self::Uniform(buffer) => buffer.update(device),
            Self::Storage(buffer) => buffer.update(device),
        }
    }
}
