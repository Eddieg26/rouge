use super::{AtomicId, Label};
use crate::core::RenderDevice;
use encase::{
    internal::{BufferMut, WriteInto, Writer},
    private::{ArrayMetadata, Metadata, RuntimeSizedArray},
    ShaderSize, ShaderType,
};
use std::{marker::PhantomData, num::NonZero, ops::Deref};
use wgpu::{util::DeviceExt, BindingResource, BufferUsages};

pub mod array;
pub mod index;
pub mod storage;
pub mod uniform;
pub mod vertex;

pub use array::*;
pub use index::*;
pub use storage::*;
pub use uniform::*;
pub use vertex::*;

pub type BufferId = AtomicId<Buffer>;

pub trait BufferData: Clone + ShaderType + ShaderSize + WriteInto + 'static {}
impl<T: Clone + ShaderType + ShaderSize + WriteInto + 'static> BufferData for T {}

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

    pub fn update(&self, device: &RenderDevice, offset: u64, data: &[u8]) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferArrayIndex<T: BufferData> {
    pub index: u32,
    pub dynamic_offset: Option<u32>,
    ty: PhantomData<T>,
}

impl<T: BufferData> BufferArrayIndex<T> {
    pub fn new(index: u32, dynamic_offset: Option<u32>) -> Self {
        Self {
            index,
            dynamic_offset,
            ty: Default::default(),
        }
    }
}

pub struct BufferArray<T: BufferData> {
    label: Label,
    data: Vec<u8>,
    buffer: Option<Buffer>,
    element_size: usize,
    usage: BufferUsages,
    is_dirty: bool,
    _phantom: PhantomData<T>,
}

impl<T: BufferData> BufferArray<T> {
    pub fn new(usage: BufferUsages) -> Self {
        Self {
            data: vec![],
            label: None,
            buffer: None,
            element_size: T::min_size().get() as usize,
            usage,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.buffer
            .as_ref()
            .map(|buffer| buffer.as_entire_binding())
    }

    pub fn element_size(&self) -> usize {
        self.element_size
    }

    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn len(&self) -> usize {
        self.data.len() / (T::min_size().get() as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity() / (T::min_size().get() as usize)
    }

    pub fn push(&mut self, value: T) -> usize {
        let offset = self.data.len();
        self.data.extend(vec![0; self.element_size]);
        let mut dst = &mut self.data[offset..offset + self.element_size];

        let mut writer = Writer::new(&value, &mut dst, 0).unwrap();
        value.write_into(&mut writer);

        offset / self.element_size
    }

    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(len * self.element_size);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.data.is_empty() {
            return;
        }

        match &self.buffer {
            Some(buffer) => {
                if self.data.len() != buffer.size() as usize {
                    let buffer =
                        Buffer::with_data(device, &self.data, self.usage, self.label.clone());
                    self.buffer = Some(buffer);
                } else if self.is_dirty {
                    buffer.update(device, 0, &self.data);
                }

                self.is_dirty = false;
            }
            None => {
                let buffer = Buffer::with_data(device, &self.data, self.usage, self.label.clone());
                self.buffer = Some(buffer);
                self.is_dirty = false;
            }
        }
    }
}

pub struct StaticArray<T> {
    array: T,
    size: NonZero<u64>,
}

impl<T> StaticArray<T> {
    pub fn new(array: T, size: NonZero<u64>) -> Self {
        Self { array, size }
    }

    pub fn size(&self) -> NonZero<u64> {
        self.size
    }
}

impl<T> std::ops::Deref for StaticArray<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.array
    }
}

impl<T> std::ops::DerefMut for StaticArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.array
    }
}

impl<T> ShaderType for StaticArray<T>
where
    T: ShaderType<ExtraMetadata = ArrayMetadata>,
{
    type ExtraMetadata = ArrayMetadata;

    const METADATA: Metadata<Self::ExtraMetadata> = T::METADATA;

    fn size(&self) -> NonZero<u64> {
        Self::METADATA.stride().mul(self.size.get()).0
    }
}

impl<T> WriteInto for StaticArray<T>
where
    T: WriteInto + RuntimeSizedArray,
{
    fn write_into<B: BufferMut>(&self, writer: &mut Writer<B>) {
        debug_assert!(self.array.len() <= self.size.get() as usize);
        self.array.write_into(writer);
    }
}
