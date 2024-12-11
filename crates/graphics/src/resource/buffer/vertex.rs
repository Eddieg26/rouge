use std::ops::RangeBounds;

use super::{Buffer, BufferSlice, Label};
use crate::core::RenderDevice;
use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

pub trait Vertex: Pod + Zeroable + 'static {}

pub struct VertexBuffer {
    inner: Buffer,
    len: u64,
}

impl VertexBuffer {
    pub fn new<V: Vertex>(
        device: &RenderDevice,
        vertices: &[V],
        usage: BufferUsages,
        label: Label,
    ) -> Self {
        let data = bytemuck::cast_slice(vertices);
        let buffer = Buffer::with_data(device, data, usage | BufferUsages::VERTEX, label);
        let len = vertices.len() as u64;

        Self { inner: buffer, len }
    }

    pub fn inner(&self) -> &Buffer {
        &self.inner
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn resize(&mut self, device: &RenderDevice, size: u64) {
        self.inner.resize(device, size);
    }

    pub fn slice<S: RangeBounds<u64>>(&self, range: S) -> BufferSlice {
        self.inner.slice(range)
    }

    pub fn update<V: Vertex>(&mut self, device: &RenderDevice, offset: u64, vertices: &[V]) {
        let data = bytemuck::cast_slice(vertices);
        self.inner.update(device, offset, data);
    }
}
