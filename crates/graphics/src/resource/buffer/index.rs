use super::{Buffer, BufferId, BufferSlice, BufferSliceId, Label};
use crate::{
    wgpu::{BufferUsages, IndexFormat},
    RenderDevice,
};
use bytemuck::{Pod, Zeroable};
use std::{marker::PhantomData, ops::RangeBounds};

pub trait Index:
    Copy + Clone + Pod + Zeroable + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static
{
    fn format() -> wgpu::IndexFormat;
}

impl Index for u32 {
    fn format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}

impl Index for u16 {
    fn format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }
}

#[derive(Clone)]
pub struct Indices<I: Index> {
    indices: Vec<I>,
}

impl<I: Index> Indices<I> {
    pub fn new(indices: Vec<I>) -> Self {
        Self { indices }
    }

    pub fn extend(&mut self, indices: Indices<I>) {
        self.indices.extend(indices.indices);
    }

    pub fn size(&self) -> u64 {
        (self.indices.len() * size_of::<I>()) as u64
    }

    pub fn format(&self) -> wgpu::IndexFormat {
        I::format()
    }
}

impl<I: Index> serde::Serialize for Indices<I> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.indices.serialize(serializer)
    }
}

impl<'de, I: Index> serde::Deserialize<'de> for Indices<I> {
    fn deserialize<D>(deserializer: D) -> Result<Indices<I>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let indices = Vec::<I>::deserialize(deserializer)?;
        Ok(Indices { indices })
    }
}

impl<I: Index> std::ops::Deref for Indices<I> {
    type Target = [I];

    fn deref(&self) -> &Self::Target {
        self.indices.as_slice()
    }
}

pub struct IndexBuffer<I: Index> {
    inner: Buffer,
    len: u64,
    _marker: PhantomData<I>,
}

impl<I: Index> IndexBuffer<I> {
    pub fn new(
        device: &RenderDevice,
        indices: Indices<I>,
        usage: BufferUsages,
        label: Label,
    ) -> Self {
        let data = bytemuck::cast_slice(&indices);
        let buffer = Buffer::with_data(device, data, usage, label);

        Self {
            inner: buffer,
            len: indices.len() as u64,
            _marker: Default::default(),
        }
    }

    pub fn inner(&self) -> &Buffer {
        &self.inner
    }

    pub fn slice<S: RangeBounds<u64>>(&self, bounds: S) -> IndexSlice {
        IndexSlice {
            format: I::format(),
            slice: self.inner.slice(bounds),
        }
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn resize(&mut self, device: &RenderDevice, size: u64) {
        self.inner.resize(device, size);
    }

    pub fn update(&mut self, device: &RenderDevice, offset: u64, indices: &Indices<I>) {
        let data = bytemuck::cast_slice(indices);
        self.inner.update(device, offset, data);
    }
}

pub struct IndexSlice<'a> {
    format: IndexFormat,
    slice: BufferSlice<'a>,
}

impl<'a> IndexSlice<'a> {
    pub fn buffer_id(&self) -> BufferId {
        self.slice.buffer_id()
    }

    pub fn id(&self) -> BufferSliceId {
        self.slice.id()
    }

    pub fn format(&self) -> IndexFormat {
        self.format
    }

    pub fn slice(&self) -> &BufferSlice<'a> {
        &self.slice
    }
}

impl<'a> std::ops::Deref for IndexSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    fn deref(&self) -> &Self::Target {
        &self.slice.slice()
    }
}
