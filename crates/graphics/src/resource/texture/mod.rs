use super::Id;
use crate::{
    wgpu::{TextureAspect, TextureFormat},
    RenderAsset, RenderDevice,
};
use std::{ops::Range, sync::Arc};

pub mod fallbacks;
pub mod render;
pub mod sampler;
pub mod target;
pub mod texture1d;
pub mod texture2d;
pub mod texture3d;
pub mod texture_cube;

pub use fallbacks::*;
pub use render::*;
pub use sampler::*;
pub use target::*;
pub use texture1d::*;
pub use texture2d::*;
pub use texture3d::*;
pub use texture_cube::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextureDimension {
    D1,
    D2,
    D2Array,
    D3,
    Cube,
    CubeArray,
}

impl Into<wgpu::TextureDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
            TextureDimension::Cube => wgpu::TextureDimension::D2,
            TextureDimension::D2Array => wgpu::TextureDimension::D2,
            TextureDimension::CubeArray => wgpu::TextureDimension::D2,
        }
    }
}

impl Into<wgpu::TextureViewDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureViewDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            TextureDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WrapMode {
    Repeat,
    ClampToEdge,
    ClampToBorder,
    MirrorRepeat,
}

impl Into<wgpu::AddressMode> for WrapMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            WrapMode::Repeat => wgpu::AddressMode::Repeat,
            WrapMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            WrapMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
            WrapMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TextureFace {
    pub start: usize,
    pub size: usize,
}

impl TextureFace {
    pub const fn new(start: usize, size: usize) -> Self {
        Self { start, size }
    }
}

pub trait Texture: 'static {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn format(&self) -> TextureFormat;
    fn dimension(&self) -> TextureDimension;
    fn filter_mode(&self) -> FilterMode;
    fn wrap_mode(&self) -> WrapMode;
    fn mipmaps(&self) -> bool;
    fn usage(&self) -> wgpu::TextureUsages;
    fn faces(&self) -> &[TextureFace];
    fn pixels(&self, range: Range<usize>) -> &[u8];
}

pub struct RenderTexture {
    texture: Arc<Option<wgpu::Texture>>,
    view: wgpu::TextureView,
}

impl RenderTexture {
    pub fn new(texture: Option<wgpu::Texture>, view: wgpu::TextureView) -> Self {
        Self {
            texture: Arc::new(texture),
            view,
        }
    }

    pub fn create<T: Texture>(device: &RenderDevice, texture: &T) -> Self {
        let size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: texture.depth(),
        };

        let mip_level_count = if texture.mipmaps() {
            let dimension = texture.dimension().into();
            size.max_mips(dimension)
        } else {
            1
        };

        let format = texture.format().into();

        let created = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: texture.width(),
                height: texture.height(),
                depth_or_array_layers: texture.depth(),
            },
            mip_level_count,
            sample_count: 1,
            dimension: texture.dimension().into(),
            format,
            usage: texture.usage(),
            view_formats: &[format],
        });

        let block_size = format.block_copy_size(None).unwrap_or(0);
        for (layer, face) in texture.faces().iter().enumerate() {
            device.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &created,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: TextureAspect::All,
                },
                texture.pixels(face.start..face.start + face.size),
                wgpu::ImageDataLayout {
                    bytes_per_row: Some(block_size * size.width),
                    rows_per_image: Some(block_size * size.width / size.height),
                    offset: 0,
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = created.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture: Arc::new(Some(created)),
            view,
        }
    }

    pub fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref().as_ref()
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

impl std::ops::Deref for RenderTexture {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl RenderAsset for RenderTexture {
    type Id = Id<RenderTexture>;
}
