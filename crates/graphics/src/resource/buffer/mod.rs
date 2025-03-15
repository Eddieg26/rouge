use super::AtomicId;
use crate::RenderDevice;
use std::ops::{Deref, RangeBounds};
use wgpu::util::DeviceExt;

pub mod index;
pub mod storage;
pub mod uniform;
pub mod vertex;

pub use index::*;
pub use storage::*;
pub use uniform::*;
pub use vertex::*;

pub type BufferId = AtomicId<Buffer>;

pub struct Buffer {
    id: BufferId,
    inner: wgpu::Buffer,
}

impl Buffer {
    pub fn new(
        device: &RenderDevice,
        size: u64,
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        });

        Self {
            id: BufferId::new(),
            inner: buffer,
        }
    }

    pub fn with_data(
        device: &RenderDevice,
        data: &[u8],
        usage: wgpu::BufferUsages,
        label: Option<&str>,
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents: data,
            usage,
        });

        Self {
            id: BufferId::new(),
            inner: buffer,
        }
    }

    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn slice<S: RangeBounds<u64>>(&self, bounds: S) -> BufferSlice {
        BufferSlice::new(self, bounds)
    }

    pub fn size(&self) -> u64 {
        self.inner.size()
    }

    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.inner.as_entire_binding()
    }

    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.inner.as_entire_buffer_binding()
    }
}

impl From<wgpu::Buffer> for Buffer {
    fn from(buffer: wgpu::Buffer) -> Self {
        Self {
            id: BufferId::new(),
            inner: buffer,
        }
    }
}

impl AsRef<wgpu::Buffer> for Buffer {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

impl AsMut<wgpu::Buffer> for Buffer {
    fn as_mut(&mut self) -> &mut wgpu::Buffer {
        &mut self.inner
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferSliceId {
    pub id: BufferId,
    pub start: u64,
    pub end: u64,
}

pub struct BufferSlice<'a> {
    id: BufferId,
    start: u64,
    end: u64,
    slice: wgpu::BufferSlice<'a>,
}

impl<'a> BufferSlice<'a> {
    fn new<S: RangeBounds<u64>>(buffer: &'a Buffer, bounds: S) -> Self {
        let start = match bounds.start_bound() {
            std::ops::Bound::Included(start) => *start,
            std::ops::Bound::Excluded(start) => start + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match bounds.end_bound() {
            std::ops::Bound::Included(end) => *end + 1,
            std::ops::Bound::Excluded(end) => *end,
            std::ops::Bound::Unbounded => buffer.size(),
        };

        Self {
            id: buffer.id(),
            start,
            end,
            slice: buffer.inner.slice(bounds),
        }
    }

    pub fn buffer_id(&self) -> BufferId {
        self.id
    }

    pub fn id(&self) -> BufferSliceId {
        BufferSliceId {
            id: self.id,
            start: self.start,
            end: self.end,
        }
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }

    pub fn slice(&self) -> &wgpu::BufferSlice<'a> {
        &self.slice
    }
}

impl<'a> Deref for BufferSlice<'a> {
    type Target = wgpu::BufferSlice<'a>;

    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}
