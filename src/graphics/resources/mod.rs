use crate::ecs::{resource::ResourceId, Resource};
use std::collections::HashMap;

use super::core::device::RenderDevice;

pub mod buffer;
pub mod material;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

pub type BufferId = ResourceId;
pub type MeshId = ResourceId;
pub type TextureId = ResourceId;
pub type SamplerId = ResourceId;
pub type MaterialId = ResourceId;
pub type ShaderId = ResourceId;
pub type ShaderGraphId = ResourceId;
pub type Resources<T> = HashMap<ResourceId, T>;

pub struct GpuResources {
    textures: Resources<wgpu::Texture>,
    views: Resources<wgpu::TextureView>,
    samplers: Resources<wgpu::Sampler>,
    uniform_buffers: Resources<wgpu::Buffer>,
    vertex_buffers: Resources<wgpu::Buffer>,
    index_buffers: Resources<wgpu::Buffer>,
    bind_groups: Resources<wgpu::BindGroup>,
    shaders: Resources<wgpu::ShaderModule>,
    defaults: DefaultResources,
}

impl GpuResources {
    pub fn new(device: &RenderDevice) -> GpuResources {
        GpuResources {
            textures: HashMap::new(),
            views: HashMap::new(),
            samplers: HashMap::new(),
            uniform_buffers: HashMap::new(),
            vertex_buffers: HashMap::new(),
            index_buffers: HashMap::new(),
            bind_groups: HashMap::new(),
            shaders: HashMap::new(),
            defaults: DefaultResources::new(device),
        }
    }

    pub fn texture(&self, id: &TextureId) -> Option<&wgpu::Texture> {
        self.textures.get(id)
    }

    pub fn texture_view(&self, id: &TextureId) -> Option<&wgpu::TextureView> {
        self.views.get(id)
    }

    pub fn sampler(&self, id: &SamplerId) -> Option<&wgpu::Sampler> {
        self.samplers.get(id)
    }

    pub fn uniform_buffer(&self, id: &BufferId) -> Option<&wgpu::Buffer> {
        self.uniform_buffers.get(id)
    }

    pub fn vertex_buffer(&self, id: &BufferId) -> Option<&wgpu::Buffer> {
        self.vertex_buffers.get(id)
    }

    pub fn index_buffer(&self, id: &BufferId) -> Option<&wgpu::Buffer> {
        self.index_buffers.get(id)
    }

    pub fn bind_group(&self, id: &BufferId) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(id)
    }

    pub fn shader(&self, id: &ShaderId) -> Option<&wgpu::ShaderModule> {
        self.shaders.get(id)
    }

    pub fn add_texture(&mut self, id: TextureId, texture: wgpu::Texture) {
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            array_layer_count: Some(texture.depth_or_array_layers()),
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            dimension: Some(match texture.dimension() {
                wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
                wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
                wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            }),
            format: Some(texture.format()),
            mip_level_count: Some(texture.mip_level_count()),
        });
        self.textures.insert(id, texture);
        self.views.insert(id, view);
    }

    pub fn add_sampler(&mut self, id: SamplerId, sampler: wgpu::Sampler) {
        self.samplers.insert(id, sampler);
    }

    pub fn add_uniform_buffer(&mut self, id: BufferId, buffer: wgpu::Buffer) {
        self.uniform_buffers.insert(id, buffer);
    }

    pub fn add_vertex_buffer(&mut self, id: BufferId, buffer: wgpu::Buffer) {
        self.vertex_buffers.insert(id, buffer);
    }

    pub fn add_index_buffer(&mut self, id: BufferId, buffer: wgpu::Buffer) {
        self.index_buffers.insert(id, buffer);
    }

    pub fn add_bind_group(&mut self, id: BufferId, bind_group: wgpu::BindGroup) {
        self.bind_groups.insert(id, bind_group);
    }

    pub fn add_shader(&mut self, id: ShaderId, shader: wgpu::ShaderModule) {
        self.shaders.insert(id, shader);
    }

    pub fn remove_texture(&mut self, id: &TextureId) -> Option<wgpu::Texture> {
        self.views.remove(id);
        self.textures.remove(id)
    }

    pub fn remove_sampler(&mut self, id: &SamplerId) -> Option<wgpu::Sampler> {
        self.samplers.remove(id)
    }

    pub fn remove_uniform_buffer(&mut self, id: &BufferId) -> Option<wgpu::Buffer> {
        self.uniform_buffers.remove(id)
    }

    pub fn remove_vertex_buffer(&mut self, id: &BufferId) -> Option<wgpu::Buffer> {
        self.vertex_buffers.remove(id)
    }

    pub fn remove_index_buffer(&mut self, id: &BufferId) -> Option<wgpu::Buffer> {
        self.index_buffers.remove(id)
    }

    pub fn remove_bind_group(&mut self, id: &BufferId) -> Option<wgpu::BindGroup> {
        self.bind_groups.remove(id)
    }

    pub fn remove_shader(&mut self, id: &ShaderId) -> Option<wgpu::ShaderModule> {
        self.shaders.remove(id)
    }
}

impl Resource for GpuResources {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct DefaultResources {
    pub texture_2d: wgpu::Texture,
    pub texture_2d_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub texture_cube: wgpu::Texture,
    pub texture_cube_view: wgpu::TextureView,
}

impl DefaultResources {
    pub fn new(device: &RenderDevice) -> DefaultResources {
        let queue = device.queue();
        let device = device.inner();

        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("White Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &white_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255u8, 255u8, 255u8],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: None,
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let white_texture_view = white_texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            array_layer_count: Some(white_texture.depth_or_array_layers()),
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            dimension: Some(match white_texture.dimension() {
                wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
                wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
                wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            }),
            format: Some(white_texture.format()),
            mip_level_count: Some(white_texture.mip_level_count()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("White Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let texture_cube = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Cube Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture_cube,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255u8, 255u8, 255u8],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: None,
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
        );

        let texture_cube_view = texture_cube.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            array_layer_count: Some(texture_cube.depth_or_array_layers()),
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            dimension: Some(match texture_cube.dimension() {
                wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
                wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
                wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
            }),
            format: Some(texture_cube.format()),
            mip_level_count: Some(texture_cube.mip_level_count()),
        });

        DefaultResources {
            texture_2d: white_texture,
            texture_2d_view: white_texture_view,
            sampler,
            texture_cube,
            texture_cube_view,
        }
    }
}
