use rouge_core::ResourceId;
use rouge_ecs::{macros::Resource, storage::sparse::SparseMap};

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
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
            dimension: Dimension::D2,
            format: Format::Rgba8UnormSrgb,
            filter_mode: FilterMode::Bilinear,
            wrap_mode: WrapMode::Clamp,
            mipmaps: false,
            pixels: Vec::new(),
        }
    }

    pub fn width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    pub fn dimension(mut self, dimension: Dimension) -> Self {
        self.dimension = dimension;
        self
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    pub fn filter_mode(mut self, filter_mode: FilterMode) -> Self {
        self.filter_mode = filter_mode;
        self
    }

    pub fn wrap_mode(mut self, wrap_mode: WrapMode) -> Self {
        self.wrap_mode = wrap_mode;
        self
    }

    pub fn mipmaps(mut self, mipmaps: bool) -> Self {
        self.mipmaps = mipmaps;
        self
    }

    pub fn pixels(mut self, pixels: Vec<u8>) -> Self {
        self.pixels = pixels;
        self
    }

    pub fn descriptor(&self) -> wgpu::TextureDescriptor {
        wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.depth,
            },
            mip_level_count: if self.mipmaps { 1 } else { 0 },
            sample_count: 1,
            dimension: self.dimension,
            format: self.format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }
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

#[derive(Resource)]
pub struct TextureViews {
    views: SparseMap<ResourceId, wgpu::TextureView>,
}

impl TextureViews {
    pub fn new() -> Self {
        Self {
            views: SparseMap::new(),
        }
    }

    pub fn insert(&mut self, id: ResourceId, view: wgpu::TextureView) {
        self.views.insert(id, view);
    }

    pub fn get(&self, id: impl Into<ResourceId>) -> Option<&wgpu::TextureView> {
        self.views.get(&id.into())
    }

    pub fn remove(&mut self, id: impl Into<ResourceId>) -> Option<wgpu::TextureView> {
        self.views.remove(&id.into())
    }

    pub fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.views.contains(&id.into())
    }

    pub fn clear(&mut self) {
        self.views.clear();
    }
}

#[derive(Resource)]
pub struct Samplers {
    samplers: SparseMap<ResourceId, wgpu::Sampler>,
}

impl Samplers {
    pub fn new() -> Self {
        Self {
            samplers: SparseMap::new(),
        }
    }

    pub fn insert(&mut self, id: ResourceId, sampler: wgpu::Sampler) {
        self.samplers.insert(id, sampler);
    }

    pub fn get(&self, id: impl Into<ResourceId>) -> Option<&wgpu::Sampler> {
        self.samplers.get(&id.into())
    }

    pub fn remove(&mut self, id: impl Into<ResourceId>) -> Option<wgpu::Sampler> {
        self.samplers.remove(&id.into())
    }

    pub fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.samplers.contains(&id.into())
    }

    pub fn clear(&mut self) {
        self.samplers.clear();
    }
}

#[derive(Resource)]
pub struct DepthTextures {
    depths: SparseMap<u32, usize>,
}

impl DepthTextures {
    pub const ROOT_NAME: &'static str = "Depth_Texture_";

    pub fn new() -> Self {
        Self {
            depths: SparseMap::new(),
        }
    }

    pub fn texture_id(depth: u32) -> ResourceId {
        ResourceId::from(Self::ROOT_NAME.to_string() + &depth.to_string())
    }

    pub fn insert(&mut self, depth: u32) {
        if let Some(count) = self.depths.get_mut(&depth) {
            *count += 1;
        } else {
            self.depths.insert(depth, 1);
        }
    }

    pub fn remove(&mut self, depth: u32) {
        self.depths.remove(&depth);
    }

    pub fn contains(&self, depth: u32) -> bool {
        self.depths.contains(&depth)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &usize)> {
        self.depths.iter()
    }

    pub fn reset(&mut self) {
        for (_, count) in self.depths.iter_mut() {
            *count = 0;
        }
    }

    pub fn retain(&mut self) -> Vec<u32> {
        let mut depths = Vec::new();

        self.depths.retain(|depth, count| {
            if *count == 0 {
                depths.push(*depth);
                false
            } else {
                true
            }
        });

        depths
    }
}
