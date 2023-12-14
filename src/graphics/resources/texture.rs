pub type Dimension = wgpu::TextureDimension;
pub type Format = wgpu::TextureFormat;

pub trait BytesPerRow {
    fn bytes_per_row(&self) -> u32;
}

impl BytesPerRow for Format {
    fn bytes_per_row(&self) -> u32 {
        match self {
            Format::R8Unorm
            | Format::R8Snorm
            | Format::R8Uint
            | Format::R8Sint
            | Format::Rg8Unorm
            | Format::Rg8Snorm
            | Format::Rg8Uint
            | Format::Rg8Sint
            | Format::Stencil8 => 1,

            Format::R16Uint
            | Format::R16Sint
            | Format::R16Float
            | Format::R16Unorm
            | Format::R16Snorm
            | Format::Depth16Unorm => 2,

            Format::Depth24Plus => 3,

            Format::Depth32Float
            | Format::EacR11Unorm
            | Format::EacR11Snorm
            | Format::Etc2Rgb8Unorm
            | Format::Etc2Rgb8UnormSrgb
            | Format::Etc2Rgb8A1Unorm
            | Format::Etc2Rgb8A1UnormSrgb
            | Format::Depth24PlusStencil8
            | Format::R32Uint
            | Format::R32Sint
            | Format::R32Float
            | Format::Rg16Uint
            | Format::Rg16Sint
            | Format::Rg16Float
            | Format::Rgba8Unorm
            | Format::Rgba8UnormSrgb
            | Format::Rgba8Snorm
            | Format::Rgba8Uint
            | Format::Rgba8Sint
            | Format::Bgra8Unorm
            | Format::Bgra8UnormSrgb
            | Format::Rgb10a2Unorm
            | Format::Rg11b10Float
            | Format::Rg16Unorm
            | Format::Rg16Snorm
            | Format::Rgb9e5Ufloat
            | Format::Bc1RgbaUnorm
            | Format::Bc1RgbaUnormSrgb
            | Format::Bc4RUnorm
            | Format::Bc4RSnorm
            | Format::Rgb10a2Uint => 4,

            Format::Depth32FloatStencil8 => 5,

            Format::Rg32Uint
            | Format::Rg32Sint
            | Format::Rg32Float
            | Format::Rgba16Uint
            | Format::Rgba16Sint
            | Format::Rgba16Float
            | Format::Rgba16Unorm
            | Format::Rgba16Snorm
            | Format::EacRg11Unorm
            | Format::EacRg11Snorm => 8,

            Format::Bc2RgbaUnorm
            | Format::Bc3RgbaUnorm
            | Format::Bc5RgUnorm
            | Format::Bc6hRgbUfloat
            | Format::Bc7RgbaUnorm
            | Format::Bc5RgSnorm
            | Format::Bc7RgbaUnormSrgb
            | Format::Rgba32Uint
            | Format::Rgba32Sint
            | Format::Rgba32Float
            | Format::Bc2RgbaUnormSrgb
            | Format::Bc3RgbaUnormSrgb
            | Format::Bc6hRgbFloat
            | Format::Etc2Rgba8Unorm
            | Format::Etc2Rgba8UnormSrgb => 16,

            Format::Astc { channel, .. } => {
                let bytes_per_block = 16;
                let bytes_per_channel = match channel {
                    wgpu::AstcChannel::Unorm => 1,
                    wgpu::AstcChannel::UnormSrgb => 1,
                    wgpu::AstcChannel::Hdr => 2,
                };
                bytes_per_block * bytes_per_channel
            }
        }
    }
}

pub trait ToTextureViewDimension {
    fn to_texture_view_dimension(&self) -> wgpu::TextureViewDimension;
}

impl ToTextureViewDimension for Dimension {
    fn to_texture_view_dimension(&self) -> wgpu::TextureViewDimension {
        match self {
            Dimension::D1 => wgpu::TextureViewDimension::D1,
            Dimension::D2 => wgpu::TextureViewDimension::D2,
            Dimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

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

pub trait Texture: 'static {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn depth(&self) -> u32;
    fn dimension(&self) -> Dimension;
    fn format(&self) -> Format;
    fn filter_mode(&self) -> FilterMode;
    fn wrap_mode(&self) -> WrapMode;
    fn mipmaps(&self) -> bool;
    fn pixels(&self) -> &[u8];
    fn view(&self) -> &wgpu::TextureView;
    fn sampler(&self) -> &wgpu::Sampler;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format: wgpu::TextureFormat,
    pub dimension: wgpu::TextureDimension,
}

impl TextureDesc {
    pub fn new_2d(width: u32, height: u32, format: wgpu::TextureFormat) -> TextureDesc {
        TextureDesc {
            width,
            height,
            depth: 1,
            format,
            dimension: wgpu::TextureDimension::D2,
        }
    }

    pub fn new_3d(width: u32, height: u32, depth: u32, format: wgpu::TextureFormat) -> TextureDesc {
        TextureDesc {
            width,
            height,
            depth,
            format,
            dimension: wgpu::TextureDimension::D3,
        }
    }

    pub fn new_cube(width: u32, height: u32, format: wgpu::TextureFormat) -> TextureDesc {
        TextureDesc {
            width,
            height,
            depth: 1,
            format,
            dimension: wgpu::TextureDimension::D2,
        }
    }
}

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
    pub fn white(width: u32, height: u32) -> Self {
        let format = Format::Rgba8UnormSrgb;
        let bytes_per_row = format.bytes_per_row();
        let pixels = vec![255; (width * height * bytes_per_row) as usize];

        Self {
            width,
            height,
            depth: 1,
            dimension: Dimension::D2,
            format,
            filter_mode: FilterMode::Bilinear,
            wrap_mode: WrapMode::Clamp,
            mipmaps: false,
            pixels,
        }
    }

    pub fn black(width: u32, height: u32) -> Self {
        let format = Format::Rgba8UnormSrgb;
        let bytes_per_row = format.bytes_per_row();
        let pixels = vec![0; (width * height * bytes_per_row) as usize];

        Self {
            width,
            height,
            depth: 1,
            dimension: Dimension::D2,
            format,
            filter_mode: FilterMode::Bilinear,
            wrap_mode: WrapMode::Clamp,
            mipmaps: false,
            pixels,
        }
    }

    pub fn gray(width: u32, height: u32) -> Self {
        let format = Format::Rgba8UnormSrgb;
        let bytes_per_row = format.bytes_per_row();
        let pixels = vec![128; (width * height * bytes_per_row) as usize];

        Self {
            width,
            height,
            depth: 1,
            dimension: Dimension::D2,
            format,
            filter_mode: FilterMode::Point,
            wrap_mode: WrapMode::Clamp,
            mipmaps: false,
            pixels,
        }
    }

    pub fn red(width: u32, height: u32) -> Self {
        let format = Format::Rgba8UnormSrgb;
        let bytes_per_row = format.bytes_per_row();
        let mut pixels = vec![0; (width * height * bytes_per_row) as usize];
        for x in 0..width {
            for y in 0..height {
                let index = (x + y * width) as usize;
                pixels[index * 4 + 0] = 255;
                pixels[index * 4 + 1] = 0;
                pixels[index * 4 + 2] = 0;
                pixels[index * 4 + 3] = 255;
            }
        }

        Self {
            width,
            height,
            depth: 1,
            dimension: Dimension::D2,
            format,
            filter_mode: FilterMode::Bilinear,
            wrap_mode: WrapMode::Clamp,
            mipmaps: false,
            pixels,
        }
    }
}

pub struct Texture2d {
    width: u32,
    height: u32,
    format: Format,
    filter_mode: FilterMode,
    wrap_mode: WrapMode,
    mipmaps: bool,
    pixels: Vec<u8>,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl Texture2d {
    pub fn new(
        width: u32,
        height: u32,
        format: Format,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
        mipmaps: bool,
        pixels: Vec<u8>,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
    ) -> Texture2d {
        Texture2d {
            width,
            height,
            format,
            filter_mode,
            wrap_mode,
            mipmaps,
            pixels,
            view,
            sampler,
        }
    }
    pub fn from_texture(
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        filter_mode: FilterMode,
        wrap_mode: WrapMode,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wrap_mode.into(),
            address_mode_v: wrap_mode.into(),
            address_mode_w: wrap_mode.into(),
            mag_filter: filter_mode.into(),
            min_filter: filter_mode.into(),
            mipmap_filter: filter_mode.into(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            array_layer_count: Some(texture.depth_or_array_layers()),
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(texture.format()),
            mip_level_count: Some(texture.mip_level_count()),
            label: None,
        });

        Texture2d::new(
            texture.width(),
            texture.height(),
            texture.format(),
            filter_mode,
            wrap_mode,
            texture.mip_level_count() > 1,
            vec![],
            view,
            sampler,
        )
    }

    pub fn from_info(device: &wgpu::Device, queue: &wgpu::Queue, info: &TextureInfo) -> Texture2d {
        let mip_levels = if info.mipmaps {
            let size = std::cmp::max(info.width, info.height);
            (size as f32).log2().floor() as u32 + 1
        } else {
            1
        };
        let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            dimension: wgpu::TextureDimension::D2,
            format: info.format,
            label: None,
            mip_level_count: mip_levels,
            sample_count: 1,
            size: wgpu::Extent3d {
                depth_or_array_layers: 1,
                height: info.height,
                width: info.width,
            },
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &gpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &info.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(info.format.bytes_per_row() * info.width),
                rows_per_image: Some(info.height),
            },
            wgpu::Extent3d {
                depth_or_array_layers: info.depth,
                height: info.height,
                width: info.width,
            },
        );

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor {
            array_layer_count: Some(info.depth),
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            dimension: Some(match info.dimension {
                wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
                wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
                wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            }),
            format: Some(info.format),
            mip_level_count: Some(mip_levels),
            label: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: info.wrap_mode.into(),
            address_mode_v: info.wrap_mode.into(),
            address_mode_w: info.wrap_mode.into(),
            mag_filter: info.filter_mode.into(),
            min_filter: info.filter_mode.into(),
            mipmap_filter: info.filter_mode.into(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Texture2d::new(
            info.width,
            info.height,
            info.format,
            info.filter_mode,
            info.wrap_mode,
            info.mipmaps,
            info.pixels.clone(),
            view,
            sampler,
        )
    }

    pub fn from_desc(device: &wgpu::Device, desc: &TextureDesc) -> Texture2d {
        let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            dimension: desc.dimension,
            format: desc.format,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                depth_or_array_layers: desc.depth,
                height: desc.height,
                width: desc.width,
            },
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Texture2d::new(
            desc.width,
            desc.height,
            desc.format,
            FilterMode::Bilinear,
            WrapMode::Clamp,
            false,
            vec![],
            view,
            sampler,
        )
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

    fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
