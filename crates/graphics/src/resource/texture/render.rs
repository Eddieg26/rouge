use std::ops::Range;

use super::{FilterMode, Texture, TextureDimension, TextureFormat, WrapMode};
use asset::Asset;
use wgpu::TextureUsages;

#[derive(serde::Serialize, serde::Deserialize, Asset)]
pub struct RenderTargetTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    pixels: Vec<u8>,
    faces: [super::TextureFace; 1],
}

impl RenderTargetTexture {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
    ) -> Self {
        let faces = [super::TextureFace::new(0, 0)];

        Self {
            wrap_mode,
            height,
            format,
            filter_mode,
            width,
            pixels: Vec::new(),
            faces,
        }
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

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}
