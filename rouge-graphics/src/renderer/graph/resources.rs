use crate::resources::{buffer::BaseBuffer, texture::GpuTexture};
use rouge_core::ResourceId;
use std::collections::HashMap;

pub type TextureId = ResourceId;
pub type BufferId = ResourceId;

pub struct TextureDesc {
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

pub struct GraphResources {
    textures: HashMap<ResourceId, GpuTexture>,
    buffers: HashMap<ResourceId, Box<dyn BaseBuffer>>,
    texture_descs: HashMap<ResourceId, TextureDesc>,
}

impl GraphResources {
    pub const SURFACE: &'static str = "surface";

    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            buffers: HashMap::new(),
            texture_descs: HashMap::new(),
        }
    }

    pub fn texture(&self, id: impl Into<ResourceId>) -> &GpuTexture {
        let id = id.into();
        self.textures.get(&id).expect("Texture not found")
    }

    pub fn buffer(&self, id: impl Into<ResourceId>) -> &dyn BaseBuffer {
        let id = id.into();
        self.buffers
            .get(&id)
            .map(|b| &**b)
            .expect("Buffer not found")
    }

    pub fn texture_checked(&self, id: impl Into<ResourceId>) -> Option<&GpuTexture> {
        let id = id.into();
        self.textures.get(&id)
    }

    pub fn buffer_checked(&self, id: impl Into<ResourceId>) -> Option<&dyn BaseBuffer> {
        let id = id.into();
        self.buffers.get(&id).map(|b| &**b)
    }

    pub fn import_texture(&mut self, id: impl Into<ResourceId>, texture: GpuTexture) {
        let id = id.into();
        self.textures.insert(id, texture);
    }

    pub fn import_buffer(&mut self, id: impl Into<ResourceId>, buffer: impl BaseBuffer) {
        let id = id.into();
        self.buffers.insert(id, Box::new(buffer));
    }

    pub fn create_texture(&mut self, id: impl Into<ResourceId>, desc: TextureDesc) {
        let id = id.into();
        self.texture_descs.insert(id, desc);
    }

    pub(crate) fn build(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        for (id, desc) in &self.texture_descs {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                dimension: wgpu::TextureDimension::D2,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                format: desc.format,
                usage: desc.usage,
                view_formats: &[],
            });

            let gpu_texture =
                GpuTexture::from_texture(&texture, &wgpu::TextureViewDescriptor::default());

            self.textures.insert(*id, gpu_texture);
        }
    }
}
