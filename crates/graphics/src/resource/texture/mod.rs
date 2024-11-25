use super::Id;
use crate::{
    wgpu::{TextureAspect, TextureFormat},
    RenderAsset, RenderDevice,
};
use ecs::core::resource::Resource;

pub mod render;
pub mod sampler;
pub mod target;
pub mod texture2d;

pub use render::*;
pub use sampler::*;
pub use target::*;
pub use texture2d::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextureDimension {
    D1,
    D2,
    D2Array,
    D3,
    D3Array,
    Cube,
}

impl Into<wgpu::TextureDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D2Array => wgpu::TextureDimension::D2,
            TextureDimension::Cube => wgpu::TextureDimension::D3,
            TextureDimension::D3Array => wgpu::TextureDimension::D3,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}

impl Into<wgpu::TextureViewDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureViewDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureDimension::D2Array => wgpu::TextureViewDimension::D2,
            TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureDimension::D3Array => wgpu::TextureViewDimension::Cube,
            TextureDimension::D3 => wgpu::TextureViewDimension::D3,
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
    fn pixels(&self) -> &[u8];
}

pub struct TextureDesc<'a> {
    pub label: Option<&'a str>,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mipmaps: bool,
    pub format: TextureFormat,
    pub dimension: TextureDimension,
    pub usage: wgpu::TextureUsages,
    pub pixels: Vec<u8>,
}

impl Default for TextureDesc<'_> {
    fn default() -> Self {
        Self {
            label: None,
            width: 1,
            height: 1,
            depth: 1,
            mipmaps: false,
            format: TextureFormat::Rgba8Unorm,
            dimension: TextureDimension::D2,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            pixels: vec![0, 0, 0, 0],
        }
    }
}

pub struct RenderTexture {
    texture: Option<wgpu::Texture>,
    view: wgpu::TextureView,
}

impl RenderTexture {
    pub fn new(texture: Option<wgpu::Texture>, view: wgpu::TextureView) -> Self {
        Self { texture, view }
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

        device.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &created,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            texture.pixels(),
            wgpu::ImageDataLayout {
                bytes_per_row: format
                    .block_copy_size(Some(TextureAspect::All))
                    .map(|s| s * size.width),
                ..Default::default()
            },
            size,
        );

        let view = created.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture: Some(created),
            view,
        }
    }

    pub fn from_desc(device: &RenderDevice, desc: &TextureDesc) -> Self {
        let size = wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: desc.depth,
        };

        let mip_level_count = if desc.mipmaps {
            let dimension = desc.dimension.into();
            size.max_mips(dimension)
        } else {
            1
        };

        let format = desc.format.into();

        let created = device.create_texture(&wgpu::TextureDescriptor {
            label: desc.label,
            size,
            mip_level_count,
            sample_count: 1,
            dimension: desc.dimension.into(),
            format,
            usage: desc.usage,
            view_formats: &[format],
        });

        if desc.pixels.len() >= desc.width as usize * desc.height as usize {
            device.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &created,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &desc.pixels,
                wgpu::ImageDataLayout {
                    bytes_per_row: format
                        .block_copy_size(Some(TextureAspect::All))
                        .map(|s| s * size.width),
                    ..Default::default()
                },
                size,
            );
        }

        let view = created.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture: Some(created),
            view,
        }
    }

    pub fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref()
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

pub struct Fallbacks {
    pub d1: Id<RenderTexture>,
    pub d2: Id<RenderTexture>,
    pub d2_array: Id<RenderTexture>,
    pub d3: Id<RenderTexture>,
    pub cube: Id<RenderTexture>,
    pub cube_array: Id<RenderTexture>,
    pub sampler: Id<Sampler>,
}

impl Fallbacks {
    pub fn dimension_id(&self, dimension: TextureDimension) -> Id<RenderTexture> {
        match dimension {
            TextureDimension::D1 => self.d1,
            TextureDimension::D2 => self.d2,
            TextureDimension::D2Array => self.d2_array,
            TextureDimension::D3 => self.d3,
            TextureDimension::Cube => self.cube,
            TextureDimension::D3Array => self.cube_array,
        }
    }
}

impl Resource for Fallbacks {}
