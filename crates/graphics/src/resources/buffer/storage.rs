use super::{Buffer, BufferArrayIndex, BufferData, BufferId, Label, StaticArray};
use crate::core::RenderDevice;
use encase::{
    internal::{CreateFrom, Reader, WriteInto},
    DynamicStorageBuffer, ShaderType, StorageBuffer as EncaseStorageBuffer,
};
use std::{marker::PhantomData, num::NonZero};
use wgpu::{BindingResource, BufferUsages};

pub struct StorageBuffer<B: BufferData> {
    value: B,
    data: EncaseStorageBuffer<Vec<u8>>,
    buffer: Buffer,
    is_dirty: bool,
}

impl<B: BufferData> StorageBuffer<B> {
    pub fn new(device: &RenderDevice, value: B, usage: Option<BufferUsages>, label: Label) -> Self {
        let mut data = EncaseStorageBuffer::new(vec![]);
        data.write(&value).unwrap();

        let usage = match usage {
            Some(usage) => usage | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            None => BufferUsages::STORAGE | BufferUsages::COPY_DST,
        };
        let buffer = Buffer::with_data(device, data.as_ref(), usage, label);

        Self {
            value,
            data,
            buffer,
            is_dirty: false,
        }
    }

    pub fn value(&self) -> &B {
        &self.value
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn id(&self) -> BufferId {
        self.buffer.id
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn binding(&self) -> BindingResource {
        self.buffer.as_entire_binding()
    }

    pub fn set(&mut self, value: B) {
        self.value = value;
        self.is_dirty = true;
    }

    pub fn get_mut(&mut self) -> &mut B {
        self.is_dirty = true;
        &mut self.value
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            self.data.write(&self.value).unwrap();
            self.buffer.update(device, 0, self.data.as_ref());
            self.is_dirty = false;
        }
    }
}

pub struct StorageBufferArray<B: ShaderType> {
    label: Label,
    buffer: Option<Buffer>,
    data: DynamicStorageBuffer<Vec<u8>>,
    usage: BufferUsages,
    is_dirty: bool,
    _phantom: PhantomData<B>,
}

impl<B: ShaderType> StorageBufferArray<B> {
    pub fn new() -> Self {
        Self {
            label: None,
            buffer: None,
            data: DynamicStorageBuffer::new(vec![]),
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn with_alignment(alignment: u64) -> Self {
        Self {
            label: None,
            buffer: None,
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

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
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
        self.buffer.as_ref().map(|b| b.as_entire_binding())
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match &self.buffer {
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
                    self.buffer = Some(new_buffer);
                    self.is_dirty = false;
                } else if self.is_dirty {
                    buffer.update(device, 0, self.data.as_ref());
                    self.is_dirty = false;
                }
            }
            None => {
                let buffer =
                    Buffer::with_data(device, self.data.as_ref(), self.usage, self.label.clone());
                self.buffer = Some(buffer);
                self.is_dirty = false;
            }
        }
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
