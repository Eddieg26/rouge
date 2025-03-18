use super::{FilterMode, Texture, WrapMode};
use asset::Asset;
use wgpu::TextureFormat;

#[derive(serde::Serialize, serde::Deserialize, Asset)]
pub struct RenderTexture {
    width: u32,
    height: u32,
    filter_mode: FilterMode,
    format: TextureFormat,
    wrap_mode: WrapMode,
    faces: [super::TextureFace; 1],
}

impl RenderTexture {
    pub const fn default_format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }

    pub(crate) fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = format;
    }
}

impl Texture for RenderTexture {
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

    fn dimension(&self) -> super::TextureDimension {
        super::TextureDimension::D2
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

    fn usage(&self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
    }

    fn faces(&self) -> &[super::TextureFace] {
        &self.faces
    }

    fn pixels(&self, _: std::ops::Range<usize>) -> &[u8] {
        &[]
    }
}
