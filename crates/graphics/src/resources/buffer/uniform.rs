use super::{Buffer, BufferData, BufferId, Label};
use crate::core::RenderDevice;
use wgpu::BufferUsages;

pub struct UniformBuffer<V: BufferData> {
    value: V,
    buffer: Buffer,
    is_dirty: bool,
}

impl<V: BufferData> UniformBuffer<V> {
    pub fn new(device: &RenderDevice, value: V, usage: BufferUsages, label: Label) -> Self {
        let data = bytemuck::bytes_of(&value);
        let buffer = Buffer::with_data(device, data, usage | BufferUsages::UNIFORM, label);

        Self {
            value,
            buffer,
            is_dirty: false,
        }
    }

    pub fn id(&self) -> BufferId {
        self.buffer.id()
    }

    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn set(&mut self, value: V) {
        self.value = value;
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            let data = bytemuck::bytes_of(&self.value);
            self.buffer.update(device, 0, data);
            self.is_dirty = false;
        }
    }
}

pub struct UniformBufferArray<V: BufferData> {
    data: Vec<u8>,
    buffer: Buffer,
    is_dirty: bool,
    max_amount: usize,
    alignment: usize,
    item_size: usize,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: BufferData> UniformBufferArray<V> {
    pub fn new(device: &RenderDevice, values: Vec<V>, usage: BufferUsages, label: Label) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let item_size = size_of::<V>() + (alignment - (size_of::<V>() % alignment));
        let padding = item_size - size_of::<V>();
        let max_amount = device.limits().max_uniform_buffer_binding_size as usize / size_of::<V>();
        let data = values
            .iter()
            .flat_map(|v| {
                bytemuck::bytes_of(v)
                    .iter()
                    .cloned()
                    .chain(std::iter::repeat(0).take(padding))
            })
            .collect::<Vec<u8>>();
        let buffer = Buffer::with_data(device, &data, usage, label);

        Self {
            data,
            buffer,
            max_amount,
            alignment,
            item_size,
            is_dirty: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn max_amount(&self) -> usize {
        self.max_amount
    }

    pub fn id(&self) -> BufferId {
        self.buffer.id()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn value(&self, index: usize) -> &V {
        let offset = index * self.item_size;
        let bytes = &self.data[offset..offset + size_of::<V>()];
        bytemuck::from_bytes(bytes)
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn len(&self) -> usize {
        self.data.len() / size_of::<V>()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, value: V) {
        self.data.extend_from_slice(bytemuck::bytes_of(&value));
        self.data.extend(std::iter::repeat(0).take(self.alignment));
        self.is_dirty = true;
    }

    pub fn set(&mut self, offset: usize, values: Vec<V>) {
        assert!(offset < self.len());

        for value in values {
            self.data.extend_from_slice(bytemuck::bytes_of(&value));
            self.data.extend(std::iter::repeat(0).take(self.alignment));
        }
        self.is_dirty = true;
    }

    pub fn remove(&mut self, index: usize) -> V {
        self.is_dirty = true;

        let offset = index * self.item_size;
        let bytes = self
            .data
            .drain(offset..offset + size_of::<V>())
            .collect::<Vec<_>>();

        bytemuck::pod_read_unaligned(&bytes)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&V) -> bool) {
        let mut i = 0;

        while i < self.len() {
            let (offset, retain) = {
                let offset = i * self.item_size;
                let bytes = &self.data[offset..offset + size_of::<V>()];
                let value = bytemuck::from_bytes(bytes);
                (offset, f(value))
            };

            match retain {
                true => i += 1,
                false => {
                    self.data.drain(offset..offset + self.item_size);
                }
            }
        }

        self.is_dirty = true;
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.is_dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            let amount = self.data.len().min(self.max_amount * self.item_size);
            let data = bytemuck::cast_slice(&self.data[..amount]);
            self.buffer.update(device, 0, data);
        }
    }
}

pub struct StaticUniformBuffer<V: BufferData, const N: usize> {
    data: Vec<u8>,
    buffer: Buffer,
    alignment: usize,
    item_size: usize,
    is_dirty: bool,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: BufferData, const N: usize> StaticUniformBuffer<V, N> {
    pub fn new(device: &RenderDevice, values: [V; N], usage: BufferUsages, label: Label) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let item_size = size_of::<V>() + (alignment - (size_of::<V>() % alignment));
        let padding = alignment - (size_of::<V>() % alignment);
        let data = values
            .iter()
            .flat_map(|v| {
                bytemuck::bytes_of(v)
                    .iter()
                    .cloned()
                    .chain(std::iter::repeat(0).take(padding))
            })
            .collect::<Vec<u8>>();
        let buffer = Buffer::with_data(device, &data, usage | BufferUsages::UNIFORM, label);

        Self {
            data,
            buffer,
            alignment,
            item_size,
            is_dirty: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> BufferId {
        self.buffer.id()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn value(&self, index: usize) -> &V {
        let offset = index * self.item_size;
        let bytes = &self.data[offset..offset + size_of::<V>()];
        bytemuck::from_bytes(bytes)
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn len(&self) -> usize {
        N
    }

    pub fn set(&mut self, index: usize, value: V) {
        let offset = index * self.item_size;
        self.data[offset..offset + size_of::<V>()].copy_from_slice(bytemuck::bytes_of(&value));
        self.data[offset + size_of::<V>()..offset + self.item_size - size_of::<V>()].fill(0);
        self.is_dirty = true;
    }

    pub fn replace(&mut self, values: [V; N]) {
        self.data.copy_from_slice(
            &values
                .iter()
                .flat_map(|v| {
                    bytemuck::bytes_of(v)
                        .iter()
                        .cloned()
                        .chain(std::iter::repeat(0).take(self.alignment))
                })
                .collect::<Vec<u8>>(),
        );
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        self.buffer.update(device, 0, &self.data);
    }
}

pub struct BatchedUniformBuffers<V: BufferData> {
    values: Vec<V>,
    buffers: Vec<Buffer>,
    is_dirty: bool,
    max_amount: usize,
}

impl<V: BufferData> BatchedUniformBuffers<V> {
    pub fn new(device: &RenderDevice, values: Vec<V>, usage: BufferUsages, label: Label) -> Self {
        let max_amount = device.limits().max_uniform_buffer_binding_size as usize / size_of::<V>();
        let buffers = values
            .chunks(max_amount)
            .map(|v| Buffer::with_data(device, bytemuck::cast_slice(v), usage, label.clone()))
            .collect();

        Self {
            values,
            buffers,
            max_amount,
            is_dirty: false,
        }
    }

    pub fn max_amount(&self) -> usize {
        self.max_amount
    }

    pub fn id(&self, index: usize) -> BufferId {
        self.buffers[index].id()
    }

    pub fn buffers(&self) -> &[Buffer] {
        &self.buffers
    }

    pub fn buffer(&self, index: usize) -> &Buffer {
        &self.buffers[index]
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn value(&self, index: usize) -> &V {
        &self.values[index]
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn push(&mut self, value: V) {
        self.values.push(value);
        self.is_dirty = true;
    }

    pub fn set(&mut self, index: usize, value: V) {
        self.values[index] = value;
        self.is_dirty = true;
    }

    pub fn remove(&mut self, index: usize) -> V {
        self.is_dirty = true;
        self.values.remove(index)
    }

    pub fn retain(&mut self, f: impl FnMut(&V) -> bool) {
        self.values.retain(f);
        self.is_dirty = true;
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.is_dirty = true;
    }

    pub fn update(&mut self, device: &RenderDevice) {
        if self.is_dirty {
            // let data = bytemuck::cast_slice(&self.values);

            // for (buffer, chunk) in self
            //     .buffers
            //     .iter_mut()
            //     .zip(data.chunks(self.max_amount * size_of::<V>()))
            // {
            //     buffer.update(device, 0, chunk);
            // }

            self.is_dirty = false;
        }
    }
}
