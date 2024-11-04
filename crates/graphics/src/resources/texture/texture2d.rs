use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use asset::{Asset, Settings};

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

impl Asset for Texture2d {}

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
