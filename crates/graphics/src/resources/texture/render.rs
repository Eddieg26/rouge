use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use asset::Asset;
use wgpu::TextureUsages;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RenderTargetTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    depth_format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    pixels: Vec<u8>,
}

impl RenderTargetTexture {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        depth_format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            wrap_mode,
            height,
            format,
            filter_mode,
            depth_format,
            width,
            pixels,
        }
    }

    pub fn depth_format(&self) -> TextureFormat {
        self.depth_format
    }
}

impl Texture for RenderTargetTexture {
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
        false
    }

    fn usage(&self) -> TextureUsages {
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING
    }

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

impl Asset for RenderTargetTexture {}
