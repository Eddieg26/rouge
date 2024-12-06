use encase::internal::AlignmentValue;

use super::{Buffer, BufferArrayIndex, BufferData, Label, StaticArray};
use crate::{
    encase::{
        internal::{CreateFrom, Reader, WriteInto},
        DynamicUniformBuffer, ShaderType, UniformBuffer as EncaseUniformBuffer,
    },
    wgpu::{BindingResource, BufferUsages},
    RenderDevice,
};
use std::{marker::PhantomData, num::NonZero};

pub struct UniformBuffer<T: ShaderType> {
    label: Label,
    value: T,
    data: EncaseUniformBuffer<Vec<u8>>,
    inner: Option<Buffer>,
    usage: BufferUsages,
    is_dirty: bool,
}

impl<T: ShaderType> UniformBuffer<T> {
    pub fn new(value: T) -> Self {
        Self {
            label: None,
            value,
            data: EncaseUniformBuffer::new(vec![]),
            inner: None,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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

impl<T: ShaderType + WriteInto> UniformBuffer<T> {
    pub fn with_buffer(device: &RenderDevice, value: T) -> Self {
        let mut data = EncaseUniformBuffer::new(vec![]);
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
            inner: Some(buffer),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            is_dirty: false,
        }
    }

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

pub struct UniformBufferArray<B: ShaderType> {
    label: Label,
    data: DynamicUniformBuffer<Vec<u8>>,
    inner: Option<Buffer>,
    usage: BufferUsages,
    is_dirty: bool,
    _phantom: PhantomData<B>,
}

impl<B: ShaderType> UniformBufferArray<B> {
    pub fn new() -> Self {
        Self {
            label: None,
            inner: None,
            data: DynamicUniformBuffer::new(vec![]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn with_alignment(alignment: u64) -> Self {
        Self {
            label: None,
            inner: None,
            data: DynamicUniformBuffer::new_with_alignment(vec![], alignment),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            is_dirty: false,
            _phantom: Default::default(),
        }
    }

    pub fn aligned(device: &RenderDevice) -> Self {
        let alignment =
            AlignmentValue::new(device.limits().min_uniform_buffer_offset_alignment as u64)
                .round_up(B::min_size().get());
        Self::with_alignment(alignment)
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

    pub fn min_alignment(device: &RenderDevice) -> u64 {
        AlignmentValue::new(device.limits().min_uniform_buffer_offset_alignment as u64)
            .round_up(B::min_size().get())
    }
}

impl<B: ShaderType + WriteInto> UniformBufferArray<B> {
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

impl<B: ShaderType + CreateFrom> UniformBufferArray<B> {
    pub fn get(&self, index: usize) -> B {
        let offset = index * B::min_size().get() as usize;
        let mut reader = Reader::new::<B>(self.data.as_ref(), offset).unwrap();
        B::create_from(&mut reader)
    }
}

pub struct StaticUniformBufferArray<B: BufferData, const N: usize> {
    label: Label,
    inner: Option<Buffer>,
    values: [B; N],
    data: Vec<u8>,
    usage: BufferUsages,
    is_dirty: bool,
}

impl<B: BufferData, const N: usize> StaticUniformBufferArray<B, N> {
    pub fn new(values: [B; N]) -> Self {
        Self {
            label: None,
            inner: None,
            values,
            data: vec![],
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn inner(&self) -> Option<&Buffer> {
        self.inner.as_ref()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn len(&self) -> usize {
        N
    }

    pub fn get(&self, index: usize) -> &B {
        &self.values[index]
    }

    pub fn set(&mut self, index: usize, value: B) {
        self.values[index] = value;
        self.is_dirty = true;
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.inner.as_ref().map(|b| b.as_entire_binding())
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match &self.inner {
            Some(buffer) => {
                if self.is_dirty {
                    buffer.update(device, 0, &self.data);
                    self.is_dirty = false;
                }
            }
            None => {
                let buffer = Buffer::with_data(device, &self.data, self.usage, self.label.clone());
                self.inner = Some(buffer);
                self.is_dirty = false;
            }
        }
    }
}

pub struct BatchedUniformBuffer<B: BufferData> {
    label: Label,
    buffer: UniformBufferArray<StaticArray<Vec<B>>>,
    batch: StaticArray<Vec<B>>,
    offset: u64,
    dynamic_offset_alignment: u32,
}

impl<B: BufferData> BatchedUniformBuffer<B> {
    pub fn new(device: &RenderDevice) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment;
        let batch_size = alignment as u64 / B::min_size().get();

        Self {
            label: None,
            buffer: UniformBufferArray::with_alignment(alignment as u64),
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

    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    pub fn usage(&self) -> BufferUsages {
        self.buffer.usage
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
