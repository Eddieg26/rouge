use super::AtomicId;
use crate::core::RenderDevice;
use bytemuck::{Pod, Zeroable};
use std::{borrow::Cow, ops::Deref};
use wgpu::{util::DeviceExt, BufferUsages};

pub mod index;
pub mod uniform;
pub mod vertex;

pub use index::*;
pub use uniform::*;
pub use vertex::*;

pub type BufferId = AtomicId<Buffer>;
pub type Label = Option<Cow<'static, str>>;

pub trait BufferData: Pod + Zeroable {}
impl<P: Pod + Zeroable> BufferData for P {}

pub struct Buffer {
    id: BufferId,
    label: Label,
    inner: wgpu::Buffer,
}

impl Buffer {
    pub fn new(device: &RenderDevice, size: u64, usage: BufferUsages, label: Label) -> Self {
        let inner = device.create_buffer(&wgpu::BufferDescriptor {
            label: label.clone().as_deref(),
            size,
            usage,
            mapped_at_creation: false,
        });

        Self {
            inner,
            label,
            id: BufferId::new(),
        }
    }

    pub fn with_data(
        device: &RenderDevice,
        data: &[u8],
        usage: BufferUsages,
        label: Label,
    ) -> Self {
        let inner = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: label.clone().as_deref(),
            contents: data,
            usage,
        });

        Self {
            inner,
            label,
            id: BufferId::new(),
        }
    }

    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn inner(&self) -> &wgpu::Buffer {
        &self.inner
    }

    pub fn resize(&mut self, device: &RenderDevice, size: u64) {
        if size != self.inner.size() {
            self.inner = device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label.clone().as_deref(),
                size,
                usage: self.inner.usage(),
                mapped_at_creation: false,
            });
        }
    }

    pub fn update(&mut self, device: &RenderDevice, offset: u64, data: &[u8]) {
        let size = (offset + data.len() as u64).min(self.inner.size()) as usize;
        device
            .queue
            .write_buffer(&self.inner, offset, &data[..size]);
    }
}

impl Deref for Buffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
