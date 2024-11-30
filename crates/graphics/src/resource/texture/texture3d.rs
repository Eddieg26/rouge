use super::{FilterMode, Texture, TextureDimension, WrapMode};
use std::ops::Range;
use wgpu::TextureFormat;

pub struct Texture3d {
    width: u32,
    height: u32,
    depth: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
    faces: [super::TextureFace; 1],
}

impl Texture3d {
    pub fn new(
        width: u32,
        height: u32,
        depth: u32,
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
            depth,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
            faces,
        }
    }
}

impl Default for Texture3d {
    fn default() -> Self {
        Self::new(
            1,
            1,
            1,
            TextureFormat::Rgba8Unorm,
            FilterMode::Linear,
            WrapMode::ClampToEdge,
            false,
            vec![0; 4],
        )
    }
}

impl Texture for Texture3d {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        self.depth
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::D3
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
