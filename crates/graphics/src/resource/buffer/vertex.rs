use super::{Buffer, BufferSlice};
use crate::RenderDevice;
use bytemuck::{Pod, Zeroable};
use std::ops::RangeBounds;
use wgpu::{BufferAddress, BufferUsages};

pub struct VertexBuffer {
    inner: Option<Buffer>,
    usage: BufferUsages,
    len: usize,
}

impl VertexBuffer {
    pub fn new<T: Pod + Zeroable>(
        device: &RenderDevice,
        vertices: &[T],
        usage: Option<BufferUsages>,
    ) -> Self {
        let usage = match usage {
            Some(usage) => usage | BufferUsages::INDEX,
            None => BufferUsages::INDEX,
        };

        Self {
            inner: if vertices.len() > 0 {
                Some(Buffer::with_data(
                    device,
                    bytemuck::cast_slice(vertices),
                    usage,
                    None,
                ))
            } else {
                None
            },
            usage,
            len: vertices.len(),
        }
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.inner.as_ref()
    }

    pub fn slice<S: RangeBounds<BufferAddress>>(&self, bounds: S) -> Option<BufferSlice> {
        self.buffer().map(|buffer| buffer.slice(bounds))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn update<T: Pod + Zeroable>(&mut self, device: &RenderDevice, vertices: &[T]) {
        if vertices.len() > 0 {
            if let Some(buffer) = &mut self.inner {
                let size = vertices.len() * std::mem::size_of::<T>();
                if size > buffer.size() as usize {
                    *buffer = Buffer::with_data(
                        device,
                        bytemuck::cast_slice(vertices),
                        buffer.as_ref().usage(),
                        None,
                    );
                    self.len = vertices.len();
                } else {
                    device
                        .queue
                        .write_buffer(buffer.as_ref(), 0, bytemuck::cast_slice(vertices));
                }
            } else {
                self.inner = Some(Buffer::with_data(
                    device,
                    bytemuck::cast_slice(vertices),
                    self.usage,
                    None,
                ));
                self.len = vertices.len();
            }
        } else {
            self.inner = None;
            self.len = 0;
        }
    }
}
