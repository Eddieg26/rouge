use super::ResourceId;
use rouge_ecs::{macros::Resource, storage::sparse::SparseMap, world::resource::Resource};

pub type Dimension = wgpu::TextureDimension;
pub type Format = wgpu::TextureFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Point,
    Bilinear,
    Trilinear,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            Self::Point => wgpu::FilterMode::Nearest,
            Self::Bilinear => wgpu::FilterMode::Linear,
            Self::Trilinear => wgpu::FilterMode::Linear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    Repeat,
    Clamp,
    Mirror,
}

impl Into<wgpu::AddressMode> for WrapMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            Self::Repeat => wgpu::AddressMode::Repeat,
            Self::Clamp => wgpu::AddressMode::ClampToEdge,
            Self::Mirror => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

pub trait Texture {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn dimension(&self) -> Dimension;
    fn format(&self) -> Format;
    fn filter_mode(&self) -> FilterMode;
    fn wrap_mode(&self) -> WrapMode;
    fn mipmaps(&self) -> bool;
    fn pixels(&self) -> &[u8];
    fn pixels_mut(&mut self) -> &mut [u8];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureInfo {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub dimension: Dimension,
    pub format: Format,
    pub filter_mode: FilterMode,
    pub wrap_mode: WrapMode,
    pub mipmaps: bool,
    pub pixels: Vec<u8>,
}

impl TextureInfo {
    pub fn new(
        width: u32,
        height: u32,
        depth: u32,
        dimension: Dimension,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            depth,
            dimension,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        }
    }

    pub fn d2(
        width: u32,
        height: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self::new(
            width,
            height,
            1,
            Dimension::D2,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        )
    }

    pub fn d3(
        width: u32,
        height: u32,
        depth: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self::new(
            width,
            height,
            depth,
            Dimension::D3,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        )
    }

    pub fn d2_array(
        width: u32,
        height: u32,
        depth: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self::new(
            width,
            height,
            depth,
            Dimension::D2,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        )
    }

    pub fn cube(
        width: u32,
        height: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self::new(
            width,
            height,
            1,
            Dimension::D2,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        )
    }

    pub fn cube_array(
        width: u32,
        height: u32,
        depth: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
    ) -> Self {
        Self::new(
            width,
            height,
            depth,
            Dimension::D2,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
        )
    }
}

pub struct Texture2D {
    width: u32,
    height: u32,
    format: Format,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
}

impl Texture2D {
    pub fn new(
        width: u32,
        height: u32,
        format: Format,
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

impl Texture for Texture2D {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn depth(&self) -> u32 {
        1
    }

    fn dimension(&self) -> Dimension {
        Dimension::D2
    }

    fn format(&self) -> Format {
        self.format
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

    fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }
}

pub struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl GpuTexture {
    pub fn new<T: Texture>(device: &wgpu::Device, texture: &T) -> Self {
        let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: texture.width(),
                height: texture.height(),
                depth_or_array_layers: texture.depth(),
            },
            mip_level_count: if texture.mipmaps() {
                1 + (texture.width() as f32).log2() as u32
            } else {
                1
            },
            sample_count: 1,
            dimension: texture.dimension(),
            format: texture.format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: texture.wrap_mode().into(),
            address_mode_v: texture.wrap_mode().into(),
            address_mode_w: texture.wrap_mode().into(),
            mag_filter: texture.filter_mode().into(),
            min_filter: texture.filter_mode().into(),
            mipmap_filter: texture.filter_mode().into(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self {
            texture: gpu_texture,
            view,
            sampler,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

#[derive(Resource)]
pub struct Textures {
    textures: SparseMap<ResourceId, GpuTexture>,
}

impl Textures {
    pub fn new() -> Self {
        Self {
            textures: SparseMap::new(),
        }
    }

    pub fn insert<T: Texture>(&mut self, device: &wgpu::Device, id: ResourceId, texture: &T) {
        let gpu_texture = GpuTexture::new(device, texture);
        self.textures.insert(id, gpu_texture);
    }

    pub fn get(&self, id: impl Into<ResourceId>) -> Option<&GpuTexture> {
        self.textures.get(&id.into())
    }

    pub fn remove(&mut self, id: impl Into<ResourceId>) -> Option<GpuTexture> {
        self.textures.remove(&id.into())
    }

    pub fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.textures.contains(&id.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ResourceId, &GpuTexture)> {
        self.textures.iter()
    }
}
