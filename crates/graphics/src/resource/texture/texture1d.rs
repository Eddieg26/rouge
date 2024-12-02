use super::{FilterMode, Texture, TextureDimension, WrapMode};
use asset::Asset;
use std::ops::Range;
use wgpu::TextureFormat;

#[derive(Clone, Asset, serde::Serialize, serde::Deserialize)]
pub struct Texture1d {
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
    faces: [super::TextureFace; 1],
}

impl Texture1d {
    pub fn new(
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        let faces = [super::TextureFace::new(0, pixels.len())];

        Self {
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
            faces,
        }
    }

    pub fn color(size: u32, color: [u8; 4]) -> Self {
        let pixels = vec![color; size as usize].concat();
        Self::new(
            wgpu::TextureFormat::Rgba8Unorm,
            FilterMode::Linear,
            WrapMode::ClampToEdge,
            false,
            pixels,
        )
    }

    pub fn white(size: u32) -> Self {
        Self::color(size, [255u8, 255, 255, 255])
    }

    pub fn black(size: u32) -> Self {
        Self::color(size, [0u8, 0, 0, 255])
    }

    pub fn gray(size: u32) -> Self {
        Self::color(size, [128u8, 128, 128, 255])
    }

    pub fn red(size: u32) -> Self {
        Self::color(size, [255u8, 0, 0, 255])
    }

    pub fn green(size: u32) -> Self {
        Self::color(size, [0, 255, 0, 255])
    }

    pub fn blue(size: u32) -> Self {
        Self::color(size, [0, 0, 255, 255])
    }
}

impl Default for Texture1d {
    fn default() -> Self {
        Self::white(1)
    }
}

impl Texture for Texture1d {
    fn width(&self) -> u32 {
        self.format
            .block_copy_size(Some(wgpu::TextureAspect::All))
            .map(|s| self.pixels.len() as u32 / s)
            .unwrap_or(0)
    }

    fn height(&self) -> u32 {
        1
    }

    fn depth(&self) -> u32 {
        1
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D1
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
