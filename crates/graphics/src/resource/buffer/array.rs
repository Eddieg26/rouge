use std::ops::RangeBounds;

use super::{
    BatchedUniformBuffer, Buffer, BufferArray, BufferArrayIndex, BufferData, BufferSlice, VertexBufferArray
};
use crate::{
    resource::Label,
    wgpu::{BindingResource, BufferUsages},
    RenderDevice,
};

pub enum RenderBufferArray<B: BufferData> {
    Vertex(VertexBufferArray<B>),
    Uniform(BatchedUniformBuffer<B>),
    Storage(BufferArray<B>),
}

impl<B: BufferData> RenderBufferArray<B> {
    pub fn new(device: &RenderDevice, usage: BufferUsages) -> Self {
        if usage.contains(BufferUsages::VERTEX) {
            Self::Vertex(VertexBufferArray::new(usage, None))
        } else {
            match device.limits().max_storage_buffers_per_shader_stage {
                0 => Self::Uniform(BatchedUniformBuffer::new(device).with_usage(usage)),
                _ => Self::Storage(BufferArray::new(usage)),
            }
        }
    }

    pub fn label(&self) -> &Label {
        match self {
            Self::Vertex(buffer) => buffer.label(),
            Self::Uniform(buffer) => buffer.label(),
            Self::Storage(buffer) => buffer.label(),
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        match self {
            Self::Vertex(buffer) => buffer.inner(),
            Self::Uniform(buffer) => buffer.inner(),
            Self::Storage(buffer) => buffer.inner(),
        }
    }

    pub fn usage(&self) -> BufferUsages {
        match self {
            Self::Vertex(buffer) => buffer.usage(),
            Self::Uniform(buffer) => buffer.usage(),
            Self::Storage(buffer) => buffer.usage(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Vertex(buffer) => buffer.is_empty(),
            Self::Uniform(buffer) => buffer.is_empty(),
            Self::Storage(buffer) => buffer.is_empty(),
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            Self::Vertex(buffer) => buffer.binding(),
            Self::Uniform(buffer) => buffer.binding(),
            Self::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn slice<S: RangeBounds<u64>>(&self, range: S) -> Option<BufferSlice> {
        match self {
            Self::Vertex(buffer) => buffer.slice(range),
            Self::Uniform(buffer) => buffer.slice(range),
            Self::Storage(buffer) => buffer.slice(range),
        }
    }

    pub fn push(&mut self, value: B) -> BufferArrayIndex<B> {
        match self {
            Self::Vertex(buffer) => buffer.push(value),
            Self::Uniform(buffer) => buffer.push(value),
            Self::Storage(buffer) => BufferArrayIndex::new(buffer.push(value) as u32, None),
        }
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match self {
            Self::Vertex(buffer) => buffer.update(device),
            Self::Uniform(buffer) => buffer.update(device),
            Self::Storage(buffer) => buffer.update(device),
        }
    }
}
