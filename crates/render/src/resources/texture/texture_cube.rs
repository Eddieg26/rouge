use std::ops::Range;

use super::{FilterMode, Texture, TextureDimension, TextureFace, WrapMode};
use asset::Asset;
use wgpu::TextureFormat;

#[derive(Clone, Asset, serde::Serialize, serde::Deserialize)]
pub struct TextureCube {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    faces: [TextureFace; 6],
    pixels: Vec<u8>,
}

impl TextureCube {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        faces: [TextureFace; 6],
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            faces,
            pixels,
        }
    }

    pub fn face(&self, face: usize) -> &[u8] {
        let face = &self.faces[face];
        &self.pixels[face.start..face.start + face.size]
    }
}

impl Default for TextureCube {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            filter_mode: FilterMode::Linear,
            wrap_mode: WrapMode::ClampToEdge,
            mipmaps: false,
            faces: [TextureFace::new(0, 4); 6],
            pixels: vec![0; 4],
        }
    }
}

impl Texture for TextureCube {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        6
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::Cube
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
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC
    }

    fn faces(&self) -> &[TextureFace] {
        &self.faces
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}

pub struct TextureCubeArray {
    width: u32,
    height: u32,
    format: TextureFormat,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    faces: Vec<[TextureFace; 6]>,
    pixels: Vec<u8>,
}

impl TextureCubeArray {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        faces: Vec<[TextureFace; 6]>,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            faces,
            pixels,
        }
    }

    pub fn face(&self, layer: usize, face: usize) -> &[u8] {
        let face = &self.faces[layer][face];
        &self.pixels[face.start..face.start + face.size]
    }
}

impl Default for TextureCubeArray {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            filter_mode: FilterMode::Linear,
            wrap_mode: WrapMode::ClampToEdge,
            mipmaps: false,
            faces: vec![[TextureFace::new(0, 4); 6]],
            pixels: vec![0; 4],
        }
    }
}

impl Texture for TextureCubeArray {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        self.faces.len() as u32 * 6
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn dimension(&self) -> TextureDimension {
        TextureDimension::CubeArray
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
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
    }

    fn faces(&self) -> &[TextureFace] {
        self.faces.as_flattened()
    }

    fn pixels(&self, range: Range<usize>) -> &[u8] {
        &self.pixels[range]
    }
}
