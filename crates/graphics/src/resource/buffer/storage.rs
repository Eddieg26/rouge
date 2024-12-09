use super::{Buffer, BufferArrayIndex, BufferData, Label, StaticArray};
use crate::{
    core::RenderDevice,
    encase::{
        internal::{CreateFrom, Reader, WriteInto},
        DynamicStorageBuffer, ShaderType, StorageBuffer as EncaseStorageBuffer,
    },
    wgpu::{BindingResource, BufferUsages},
};
use std::{marker::PhantomData, num::NonZero};

pub struct StorageBuffer<T: ShaderType> {
    label: Label,
    value: T,
    data: EncaseStorageBuffer<Vec<u8>>,
    inner: Buffer,
    usage: BufferUsages,
    is_dirty: bool,
}

impl<T: ShaderType> StorageBuffer<T> {
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn inner(&self) -> &Buffer {
        &self.inner
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn binding(&self) -> BindingResource {
        self.inner.as_entire_binding()
    }
}

impl<T: ShaderType + WriteInto> StorageBuffer<T> {
    pub fn new(device: &RenderDevice, value: T) -> Self {
        let mut data = EncaseStorageBuffer::new(vec![]);
        data.write(&value).unwrap();

        let buffer = Buffer::with_data(
            device,
            data.as_ref(),
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            None,
        );

        Self {
            label: None,
            value,
            data,
            inner: buffer,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            is_dirty: false,
        }
    }
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            self.data.write(&self.value).unwrap();
            self.inner.update(device, 0, self.data.as_ref());
            self.is_dirty = false;
        }
    }
}

pub struct LazyStorageBuffer<T: ShaderType> {
    label: Label,
    value: T,
    data: EncaseStorageBuffer<Vec<u8>>,
    inner: Option<Buffer>,
    usage: BufferUsages,
    is_dirty: bool,
}

impl<T: ShaderType> LazyStorageBuffer<T> {
    pub fn new(value: T) -> Self {
        Self {
            label: None,
            value,
            data: EncaseStorageBuffer::new(vec![]),
            inner: None,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            is_dirty: false,
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn inner(&self) -> Option<&Buffer> {
        self.inner.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.inner.as_ref().map(|b| b.as_entire_binding())
    }
}

impl<T: ShaderType + WriteInto> LazyStorageBuffer<T> {
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match &self.inner {
            Some(buffer) if self.is_dirty => {
                self.data.write(&self.value).unwrap();
                buffer.update(device, 0, self.data.as_ref());
                self.is_dirty = false;
            }
            None => {
                self.data.write(&self.value).unwrap();
                let buffer =
                    Buffer::with_data(device, self.data.as_ref(), self.usage, self.label.clone());
                self.inner = Some(buffer);
                self.is_dirty = false;
            }
            _ => {}
        }
    }
}

pub struct StorageBufferArray<B: ShaderType> {
    label: Label,
    data: DynamicStorageBuffer<Vec<u8>>,
    inner: Option<Buffer>,
    usage: BufferUsages,
    is_dirty: bool,
    _phantom: PhantomData<B>,
}

impl<B: ShaderType> StorageBufferArray<B> {
    pub fn new() -> Self {
        Self {
            label: None,
            inner: None,
            data: DynamicStorageBuffer::new(vec![]),
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn with_alignment(alignment: u64) -> Self {
        Self {
            label: None,
            inner: None,
            data: DynamicStorageBuffer::new_with_alignment(vec![], alignment),
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn inner(&self) -> Option<&Buffer> {
        self.inner.as_ref()
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn len(&self) -> usize {
        self.data.as_ref().len() / (B::min_size().get() as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.data.as_ref().is_empty()
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.inner.as_ref().map(|b| b.as_entire_binding())
    }
}

impl<B: ShaderType + WriteInto> StorageBufferArray<B> {
    pub fn push(&mut self, value: &B) {
        self.data.write(value).unwrap();
        self.is_dirty = true;
    }

    pub fn set(&mut self, index: usize, value: B) {
        self.data.set_offset(index as u64 * B::min_size().get());
        self.data.write(&value).unwrap();
        self.data.set_offset(self.data.as_ref().len() as u64);
        self.is_dirty = true;
    }

    pub fn clear(&mut self) {
        self.data.as_mut().clear();
        self.data.set_offset(0);
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match &self.inner {
            Some(buffer) => {
                let capacity = buffer.size();
                let size = self.data.as_ref().len() as u64;

                if size > capacity {
                    let new_buffer = Buffer::with_data(
                        device,
                        self.data.as_ref(),
                        self.usage,
                        self.label.clone(),
                    );
                    self.inner = Some(new_buffer);
                    self.is_dirty = false;
                } else if self.is_dirty {
                    buffer.update(device, 0, self.data.as_ref());
                    self.is_dirty = false;
                }
            }
            None => {
                let buffer =
                    Buffer::with_data(device, self.data.as_ref(), self.usage, self.label.clone());
                self.inner = Some(buffer);
                self.is_dirty = false;
            }
        }
    }
}

impl<B: ShaderType + CreateFrom> StorageBufferArray<B> {
    pub fn get(&self, index: usize) -> B {
        let offset = index * B::min_size().get() as usize;
        let mut reader = Reader::new::<B>(self.data.as_ref(), offset).unwrap();
        B::create_from(&mut reader)
    }
}

pub struct BatchedStorageBuffer<B: BufferData> {
    label: Label,
    buffer: StorageBufferArray<StaticArray<Vec<B>>>,
    batch: StaticArray<Vec<B>>,
    offset: u64,
    dynamic_offset_alignment: u32,
}

impl<B: BufferData> BatchedStorageBuffer<B> {
    pub fn new(device: &RenderDevice) -> Self {
        let alignment = device.limits().min_storage_buffer_offset_alignment;
        let batch_size = alignment as u64 / B::min_size().get();

        Self {
            label: None,
            buffer: StorageBufferArray::with_alignment(alignment as u64),
            batch: StaticArray::new(vec![], NonZero::new(batch_size).unwrap()),
            offset: 0,
            dynamic_offset_alignment: alignment,
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.buffer.usage = usage;
        self
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn size(&self) -> NonZero<u64> {
        self.batch.size()
    }

    pub fn inner(&self) -> Option<&Buffer> {
        self.buffer.inner()
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.buffer.binding()
    }

    pub fn push(&mut self, value: B) -> BufferArrayIndex<B> {
        let index = BufferArrayIndex::new(self.batch.len() as u32, Some(self.offset as u32));
        self.batch.push(value);
        if self.batch.len() == self.size().get() as usize {
            self.flush();
        }

        index
    }

    pub fn flush(&mut self) {
        self.buffer.push(&self.batch);
        self.align_offset();
        self.batch.clear();
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if !self.batch.is_empty() {
            self.flush();
        }

        self.buffer.update(device);
    }

    fn align_offset(&mut self) {
        let alignment = self.dynamic_offset_alignment as u64;
        let offset = self.offset;
        self.offset = (offset + alignment - 1) & !(alignment - 1);
    }
}
