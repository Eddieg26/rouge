use super::Buffer;
use crate::RenderDevice;
use encase::{
    internal::{AlignmentValue, WriteInto},
    ShaderType,
};
use wgpu::{BindingResource, BufferUsages, Limits};

pub struct UniformBufferDesc<T: ShaderType + WriteInto> {
    pub value: T,
    pub label: Option<String>,
    pub usage: Option<wgpu::BufferUsages>,
}

pub struct UniformBuffer<T: ShaderType + WriteInto> {
    value: T,
    data: encase::UniformBuffer<Vec<u8>>,
    buffer: Buffer,
    is_dirty: bool,
}

impl<T: ShaderType + WriteInto> UniformBuffer<T> {
    pub fn new(device: &RenderDevice, desc: UniformBufferDesc<T>) -> Self {
        let mut data = encase::UniformBuffer::new(Vec::with_capacity(T::min_size().get() as usize));
        data.write(&desc.value).unwrap();

        let usage = match desc.usage {
            Some(usage) => usage | BufferUsages::UNIFORM,
            None => BufferUsages::UNIFORM,
        };

        let buffer = Buffer::with_data(
            device,
            data.as_ref(),
            usage,
            desc.label.as_ref().map(String::as_str),
        );

        Self {
            value: desc.value,
            data,
            buffer,
            is_dirty: false,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn binding(&self) -> BindingResource {
        self.buffer.as_entire_binding()
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            device
                .queue
                .write_buffer(self.buffer.as_ref(), 0, self.data.as_ref());
            self.is_dirty = false;
        }
    }
}

pub struct UniformBufferArray<T: ShaderType + WriteInto> {
    data: encase::DynamicUniformBuffer<Vec<u8>>,
    buffer: Option<Buffer>,
    label: Option<String>,
    usage: wgpu::BufferUsages,
    is_dirty: bool,
    alignment: wgpu::BufferAddress,
    _marker: std::marker::PhantomData<T>,
}

impl<T: ShaderType + WriteInto> UniformBufferArray<T> {
    pub fn new() -> Self {
        Self::new_with_alignment(T::min_size().get())
    }

    pub fn aligned(limits: &Limits) -> Self {
        let alignment = AlignmentValue::new(T::min_size().get())
            .round_up(limits.min_uniform_buffer_offset_alignment as u64);

        Self::new_with_alignment(alignment)
    }

    pub fn new_with_alignment(alignment: u64) -> Self {
        Self {
            data: encase::DynamicUniformBuffer::new_with_alignment(Vec::new(), alignment),
            buffer: None,
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            is_dirty: true,
            alignment,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage = usage | wgpu::BufferUsages::UNIFORM;
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn usage(&self) -> wgpu::BufferUsages {
        self.usage
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.buffer
            .as_ref()
            .map(|buffer| buffer.as_entire_binding())
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref().as_slice()
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn alignment(&self) -> wgpu::BufferAddress {
        self.alignment
    }

    pub fn len(&self) -> usize {
        self.data.as_ref().len() / self.alignment as usize
    }

    pub fn push(&mut self, value: T) -> u64 {
        self.is_dirty = true;
        self.data.write(&value).unwrap()
    }

    pub fn set(&mut self, index: usize, values: impl IntoIterator<Item = T>) -> Vec<u64> {
        self.is_dirty = true;
        self.data
            .set_offset(index as wgpu::BufferAddress * self.alignment);

        let offsets = values
            .into_iter()
            .map(|value| self.data.write(&value).unwrap())
            .collect();

        self.data.set_offset(self.data.as_ref().len() as u64);

        offsets
    }

    pub fn update(&mut self, device: &RenderDevice) {
        match &mut self.buffer {
            Some(buffer) if self.is_dirty => {
                device
                    .queue
                    .write_buffer(buffer.as_ref(), 0, self.data.as_ref());
                self.is_dirty = false;
            }
            None if !self.data.as_ref().is_empty() => {
                self.buffer = Some(Buffer::with_data(
                    device,
                    self.data.as_ref(),
                    self.usage,
                    self.label.as_ref().map(String::as_str),
                ));

                self.is_dirty = false;
            }
            _ => (),
        }
    }
}
