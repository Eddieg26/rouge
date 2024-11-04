use crate::core::RenderDevice;
use spatial::size::Size;
use std::collections::HashMap;
use wgpu::TextureFormat;

pub use wgpu::{BufferUsages, TextureUsages};

pub trait GraphResource: Sized + 'static {
    fn id(value: &str) -> GraphResourceId;
}

#[derive(Debug, Clone, Copy)]
pub struct TextureDesc {
    pub format: TextureFormat,
    pub usages: TextureUsages,
}

pub struct RenderGraphTexture(wgpu::TextureView);
impl RenderGraphTexture {
    pub fn create(device: &RenderDevice, desc: &TextureDesc, width: u32, height: u32) -> Self {
        Self(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format.into(),
                    usage: desc.usages,
                    view_formats: &[desc.format.into()],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        )
    }
}

impl GraphResource for RenderGraphTexture {
    fn id(value: &str) -> GraphResourceId {
        GraphResourceId::Texture(Id::new(value))
    }
}

impl std::ops::Deref for RenderGraphTexture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct BufferDesc {
    pub usage: BufferUsages,
    pub size: u64,
}

pub struct RenderGraphBuffer(wgpu::Buffer);
impl RenderGraphBuffer {
    pub fn create(device: &RenderDevice, desc: &BufferDesc) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: desc.size,
            usage: desc.usage,
            mapped_at_creation: false,
        });

        Self(buffer)
    }
}

impl GraphResource for RenderGraphBuffer {
    fn id(value: &str) -> GraphResourceId {
        GraphResourceId::Buffer(Id::new(value))
    }
}

impl std::ops::Deref for RenderGraphBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct Id<T> {
    id: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(name: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(name.as_bytes());
        Self {
            id: hasher.finalize(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.id)
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.id)
    }
}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Id<T> {
    pub fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphResourceId {
    Texture(Id<RenderGraphTexture>),
    Buffer(Id<RenderGraphBuffer>),
}

impl From<Id<RenderGraphTexture>> for GraphResourceId {
    fn from(id: Id<RenderGraphTexture>) -> Self {
        Self::Texture(id)
    }
}

impl From<Id<RenderGraphBuffer>> for GraphResourceId {
    fn from(id: Id<RenderGraphBuffer>) -> Self {
        Self::Buffer(id)
    }
}

pub struct GraphResources {
    size: Size,
    texture_descs: HashMap<Id<RenderGraphTexture>, TextureDesc>,
    buffer_descs: HashMap<Id<RenderGraphBuffer>, BufferDesc>,
    textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
    buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
}

impl GraphResources {
    pub fn new() -> Self {
        Self {
            size: Size::ZERO,
            texture_descs: HashMap::new(),
            buffer_descs: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn create_texture(&mut self, name: &str, desc: TextureDesc) -> Id<RenderGraphTexture> {
        let handle = Id::new(name);
        self.texture_descs.insert(handle, desc);
        handle
    }

    pub fn create_buffer(&mut self, name: &str, desc: BufferDesc) -> Id<RenderGraphBuffer> {
        let handle = Id::new(name);
        self.buffer_descs.insert(handle, desc);
        handle
    }

    pub fn import_texture(
        &mut self,
        name: &str,
        texture: RenderGraphTexture,
    ) -> Id<RenderGraphTexture> {
        let handle = Id::new(name);
        self.textures.insert(handle, texture);
        handle
    }

    pub fn import_buffer(
        &mut self,
        name: &str,
        buffer: RenderGraphBuffer,
    ) -> Id<RenderGraphBuffer> {
        let handle = Id::new(name);
        self.buffers.insert(handle, buffer);
        handle
    }

    pub fn remove_texture(&mut self, handle: Id<RenderGraphTexture>) {
        self.textures.remove(&handle);
    }

    pub fn remove_buffer(&mut self, handle: Id<RenderGraphBuffer>) {
        self.buffers.remove(&handle);
    }

    pub fn resize(&mut self, device: &RenderDevice, size: Size) {
        let new_size = self.size.max(size);
        if new_size.width > self.size.width || new_size.height > self.size.height {
            self.size = new_size;

            for (handle, desc) in self.texture_descs.iter() {
                if !self.textures.contains_key(handle) {
                    let texture =
                        RenderGraphTexture::create(device, desc, self.size.width, self.size.height);
                    self.textures.insert(*handle, texture);
                }
            }
        }
    }

    pub fn build(&mut self, device: &RenderDevice) {
        for (handle, desc) in self.buffer_descs.iter() {
            if !self.buffers.contains_key(handle) {
                let buffer = RenderGraphBuffer::create(device, desc);
                self.buffers.insert(*handle, buffer);
            }
        }
    }
}
