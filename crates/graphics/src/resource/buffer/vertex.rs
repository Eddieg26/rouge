use super::{Buffer, BufferArrayIndex, BufferData, BufferSlice, Label};
use crate::core::RenderDevice;
use bytemuck::{Pod, Zeroable};
use std::ops::RangeBounds;
use wgpu::BufferUsages;

pub trait Vertex: Pod + Zeroable + 'static {}
impl<P: Pod + Zeroable> Vertex for P {}

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

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn usage(&self) -> BufferUsages {
        self.inner.usage()
    }

    pub fn resize(&mut self, device: &RenderDevice, size: u64) {
        self.inner.resize(device, size);
    }

    pub fn slice<S: RangeBounds<u64>>(&self, range: S) -> BufferSlice {
        self.inner.slice(range)
    }

    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        self.inner.as_entire_binding()
    }

    pub fn update<V: Vertex>(&mut self, device: &RenderDevice, offset: u64, vertices: &[V]) {
        let data = bytemuck::cast_slice(vertices);
        self.inner.update(device, offset, data);
    }
}

pub struct VertexBufferArray<V: Vertex> {
    inner: Option<Buffer>,
    usage: BufferUsages,
    label: Label,
    vertices: Vec<V>,
    is_dirty: bool,
}

impl<V: Vertex> VertexBufferArray<V> {
    pub fn new(usage: BufferUsages, label: Label) -> Self {
        Self {
            inner: None,
            label,
            usage,
            is_dirty: false,
            vertices: vec![],
        }
    }

    pub fn inner(&self) -> Option<&Buffer> {
        self.inner.as_ref()
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn len(&self) -> u64 {
        self.vertices.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    pub fn slice<S: RangeBounds<u64>>(&self, range: S) -> Option<BufferSlice> {
        self.inner.as_ref().map(|buffer| buffer.slice(range))
    }

    pub fn binding(&self) -> Option<wgpu::BindingResource<'_>> {
        self.inner.as_ref().map(|buffer| buffer.as_entire_binding())
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match self.inner.as_mut() {
            Some(buffer) if self.is_dirty => {
                let size = (self.vertices.len() * std::mem::size_of::<V>()) as u64;
                if size != buffer.size() {
                    buffer.resize(device, size);
                }
                buffer.update(device, 0, bytemuck::cast_slice(&self.vertices));
                self.is_dirty = false;
            }
            None if !self.vertices.is_empty() => {
                let data = bytemuck::cast_slice(&self.vertices);
                let buffer = Buffer::with_data(device, data, self.usage, self.label.clone());
                self.inner = Some(buffer);
                self.is_dirty = false;
            }
            _ => {}
        }
    }
}

impl<V: Vertex + BufferData> VertexBufferArray<V> {
    pub fn push(&mut self, value: V) -> BufferArrayIndex<V> {
        self.vertices.push(value);
        self.is_dirty = true;
        BufferArrayIndex::new(self.vertices.len() as u32, None)
    }
}
