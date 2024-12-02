use super::{
    sampler::{Sampler, SamplerDesc},
    FilterMode, RenderTexture, Texture, TextureDimension, TextureFormat, WrapMode,
};
use crate::core::{RenderAssetExtractor, RenderAssets, RenderDevice};
use asset::{Asset, Settings};
use ecs::system::unlifetime::{ReadRes, WriteRes};
use std::ops::Range;

#[derive(Clone, serde::Serialize, serde::Deserialize, Asset)]
pub struct Texture2d {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
    faces: [super::TextureFace; 1],
}

impl Texture2d {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        let faces = [super::TextureFace::new(0, pixels.len())];
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
            faces,
        }
    }

    pub fn color(width: u32, height: u32, pixel: [u8; 4]) -> Self {
        let pixels = vec![pixel; (width * height) as usize].concat();
        Self::new(
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            FilterMode::Linear,
            WrapMode::ClampToEdge,
            false,
            pixels,
        )
    }

    pub fn white(width: u32, height: u32) -> Self {
        Self::color(width, height, [255u8, 255, 255, 255])
    }

    pub fn black(width: u32, height: u32) -> Self {
        Self::color(width, height, [0u8, 0, 0, 255])
    }

    pub fn gray(width: u32, height: u32) -> Self {
        Self::color(width, height, [128u8, 128, 128, 255])
    }

    pub fn red(width: u32, height: u32) -> Self {
        Self::color(width, height, [255u8, 0, 0, 255])
    }

    pub fn green(width: u32, height: u32) -> Self {
        Self::color(width, height, [0, 255, 0, 255])
    }

    pub fn blue(width: u32, height: u32) -> Self {
        Self::color(width, height, [0, 0, 255, 255])
    }
}

impl Default for Texture2d {
    fn default() -> Self {
        Self::white(1, 1)
    }
}

impl Texture for Texture2d {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        1
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D2
    }

    fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    fn mipmaps(&self) -> bool {
        self.mipmaps
    }

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
    }

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Texture2dSettings {
    pub format: TextureFormat,
    pub filter_mode: FilterMode,
    pub wrap_mode: WrapMode,
    pub mipmaps: bool,
}

impl Default for Texture2dSettings {
    fn default() -> Self {
        Self {
            format: TextureFormat::Rgba8Unorm,
            filter_mode: FilterMode::Linear,
            wrap_mode: WrapMode::Repeat,
            mipmaps: false,
        }
    }
}

impl Settings for Texture2dSettings {}

impl RenderAssetExtractor for Texture2d {
    type Source = Texture2d;
    type Asset = RenderTexture;
    type Arg = (ReadRes<RenderDevice>, WriteRes<RenderAssets<Sampler>>);

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::core::ExtractError> {
        let (device, samplers) = arg;

        let texture = RenderTexture::create(device, source);
        let sampler = Sampler::create(
            device,
            &SamplerDesc {
                label: None,
                wrap_mode: source.wrap_mode,
                filter_mode: source.filter_mode,
                border_color: match source.wrap_mode {
                    WrapMode::ClampToBorder => Some(wgpu::SamplerBorderColor::TransparentBlack),
                    _ => None,
                },
                ..Default::default()
            },
        );

        samplers.add(id.into(), sampler);

        source.pixels.clear();

        Ok(texture)
    }

    fn remove(
        id: &asset::AssetId,
        assets: &mut crate::core::RenderAssets<Self::Asset>,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) {
        let (.., samplers) = arg;

        samplers.remove(&id.into());
        assets.remove(&id.into());
    }
}

pub struct Texture2dArray {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
    faces: Vec<super::TextureFace>,
}

impl Texture2dArray {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        faces: Vec<super::TextureFace>,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
            faces,
        }
    }

    pub fn color(width: u32, height: u32, layers: u32, pixel: [u8; 4]) -> Self {
        let pixels = vec![pixel; (width * height) as usize].concat();
        let faces = vec![super::TextureFace::new(0, pixels.len()); layers as usize];
        Self::new(
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            FilterMode::Linear,
            WrapMode::ClampToEdge,
            false,
            faces,
            pixels,
        )
    }

    pub fn white(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [255u8, 255, 255, 255])
    }

    pub fn black(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [0u8, 0, 0, 255])
    }

    pub fn gray(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [128u8, 128, 128, 255])
    }

    pub fn red(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [255u8, 0, 0, 255])
    }

    pub fn green(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [0, 255, 0, 255])
    }

    pub fn blue(width: u32, height: u32, layers: u32) -> Self {
        Self::color(width, height, layers, [0, 0, 255, 255])
    }
}

impl Default for Texture2dArray {
    fn default() -> Self {
        Self::white(1, 1, 1)
    }
}

impl Texture for Texture2dArray {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        self.faces.len() as u32
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D2Array
    }

    fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }

    fn mipmaps(&self) -> bool {
        self.mipmaps
    }

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
    }

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}
