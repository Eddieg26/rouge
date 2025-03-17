use crate::{device::RenderDevice, resources::Id};
use std::collections::HashMap;
use wgpu::TextureDescriptor;

pub mod context;
pub mod graph;

pub struct TextureDesc {
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

pub struct BufferDesc {
    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
}

pub enum RenderGraphResource {
    Texture {
        id: Id<RenderGraphTexture>,
        desc: TextureDesc,
    },
    Buffer {
        id: Id<RenderGraphBuffer>,
        desc: BufferDesc,
    },
}

pub struct RenderGraphTexture {
    pub view: wgpu::TextureView,
    pub desc: Option<TextureDesc>,
}

impl std::ops::Deref for RenderGraphTexture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl AsRef<wgpu::TextureView> for RenderGraphTexture {
    fn as_ref(&self) -> &wgpu::TextureView {
        &self.view
    }
}

pub struct RenderGraphBuffer {
    pub buffer: wgpu::Buffer,
    pub desc: Option<BufferDesc>,
}

impl std::ops::Deref for RenderGraphBuffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl AsRef<wgpu::Buffer> for RenderGraphBuffer {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

pub struct RenderGraphResources {
    textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
    buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
}

impl RenderGraphResources {
    pub fn new(
        textures: HashMap<Id<RenderGraphTexture>, RenderGraphTexture>,
        buffers: HashMap<Id<RenderGraphBuffer>, RenderGraphBuffer>,
    ) -> Self {
        Self { textures, buffers }
    }

    pub fn import_texture(
        &mut self,
        id: impl Into<Id<RenderGraphTexture>>,
        texture: wgpu::TextureView,
    ) -> Id<RenderGraphTexture> {
        let id = id.into();
        self.textures.insert(
            id,
            RenderGraphTexture {
                view: texture,
                desc: None,
            },
        );

        id
    }

    pub fn import_buffer(
        &mut self,
        id: impl Into<Id<RenderGraphBuffer>>,
        buffer: wgpu::Buffer,
    ) -> Id<RenderGraphBuffer> {
        let id = id.into();
        self.buffers
            .insert(id, RenderGraphBuffer { buffer, desc: None });

        id
    }

    pub fn remove_texture(&mut self, id: &Id<RenderGraphTexture>) {
        self.textures.remove(id);
    }

    pub fn remove_buffer(&mut self, id: &Id<RenderGraphBuffer>) {
        self.buffers.remove(id);
    }

    pub fn resize(&mut self, device: &RenderDevice, width: u32, height: u32) {
        for texture in &mut self.textures.values_mut() {
            texture.view = {
                let Some(desc) = texture.desc.as_ref() else {
                    continue;
                };

                let texture = device.create_texture(&TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 0,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format,
                    usage: desc.usage,
                    view_formats: &[desc.format],
                });

                texture.create_view(&Default::default())
            };
        }
    }
}
