use super::{
    sampler::{Sampler, SamplerDesc},
    FilterMode, RenderTexture, Texture, TextureDimension, TextureFormat, WrapMode,
};
use crate::core::{RenderAssetExtractor, RenderAssets, RenderDevice};
use asset::Settings;
use ecs::system::{
    unlifetime::{ReadRes, WriteRes},
    StaticArg,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Texture2d {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
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
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        }
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
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl asset::Asset for Texture2d {}

impl std::fmt::Display for Texture2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Texture2d {{ width: {}, height: {}, format: {:?}, filter_mode: {:?}, wrap_mode: {:?}, mipmaps: {} }}",
            self.width, self.height, self.format, self.filter_mode, self.wrap_mode, self.mipmaps
        )
    }
}

impl std::fmt::Debug for Texture2d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
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
    type Arg = StaticArg<'static, (ReadRes<RenderDevice>, WriteRes<RenderAssets<Sampler>>)>;

    fn extract(
        id: &asset::AssetId,
        source: &mut Self::Source,
        arg: &mut ecs::system::ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, crate::core::ExtractError> {
        let (device, samplers) = arg.inner_mut();

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
        let (.., samplers) = arg.inner_mut();

        samplers.remove(&id.into());
        assets.remove(&id.into());
    }
}
