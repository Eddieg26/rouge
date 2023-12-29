use wgpu::util::DeviceExt;

use super::{GpuResources, SamplerId, ShaderId, TextureId};
use std::any::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderModel {
    Lit,
    Unlit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl BlendMode {
    pub fn color_target_state(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
        match self {
            BlendMode::Opaque => wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            },
            BlendMode::Transparent => wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            },
        }
    }
}

pub trait Material: 'static {
    fn fragment_shader(&self) -> ShaderId;

    fn model(&self) -> ShaderModel {
        ShaderModel::Unlit
    }

    fn blend_mode(&self) -> BlendMode {
        BlendMode::Opaque
    }

    fn properties(&self) -> MaterialProperties {
        MaterialProperties::new()
    }

    fn test() {}
}

pub struct UnlitTexture {
    texture: TextureId,
}

impl UnlitTexture {
    pub fn set_texture(&mut self, texture: TextureId) {
        self.texture = texture;
    }
}

impl Material for UnlitTexture {
    fn fragment_shader(&self) -> ShaderId {
        ShaderId::from("unlit_texture")
    }

    fn properties(&self) -> MaterialProperties {
        let mut properties = MaterialProperties::new();
        properties.add_resource(MaterialResource::Texture2D(self.texture));
        properties
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialType(TypeId);

impl MaterialType {
    pub fn new<M: Material>() -> MaterialType {
        MaterialType(TypeId::of::<M>())
    }

    pub fn id(&self) -> TypeId {
        self.0
    }
}

impl From<TypeId> for MaterialType {
    fn from(id: TypeId) -> MaterialType {
        MaterialType(id)
    }
}

impl std::ops::Deref for MaterialType {
    type Target = TypeId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum MaterialAttribute {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat4([f32; 16]),
}

impl MaterialAttribute {
    pub fn size(&self) -> usize {
        match self {
            MaterialAttribute::Float(_) => 4,
            MaterialAttribute::Vec2(_) => 8,
            MaterialAttribute::Vec3(_) => 12,
            MaterialAttribute::Vec4(_) => 16,
            MaterialAttribute::Mat4(_) => 64,
        }
    }

    pub fn padded(&self) -> Vec<f32> {
        let mut padded = vec![];
        match self {
            MaterialAttribute::Float(value) => {
                padded.push(*value);
                padded.push(0.0);
            }
            MaterialAttribute::Vec2(value) => {
                padded.extend_from_slice(value);
            }
            MaterialAttribute::Vec3(value) => {
                padded.push(value[0]);
                padded.push(value[1]);
                padded.push(value[2]);
                padded.push(0.0);
            }
            MaterialAttribute::Vec4(value) => {
                padded.extend_from_slice(value);
            }
            MaterialAttribute::Mat4(value) => {
                padded.extend_from_slice(value);
            }
        }
        padded
    }
}

pub enum MaterialResource {
    Texture2D(TextureId),
    TextureCube(TextureId),
    Sampler(SamplerId),
}

pub struct MaterialProperties {
    attributes: Vec<MaterialAttribute>,
    resources: Vec<MaterialResource>,
}

impl MaterialProperties {
    pub fn new() -> MaterialProperties {
        MaterialProperties {
            attributes: vec![],
            resources: vec![],
        }
    }

    pub fn add_attribute(&mut self, attribute: MaterialAttribute) {
        self.attributes.push(attribute);
    }

    pub fn add_resource(&mut self, resource: MaterialResource) {
        self.resources.push(resource);
    }

    pub fn attributes(&self) -> &[MaterialAttribute] {
        &self.attributes
    }

    pub fn resources(&self) -> &[MaterialResource] {
        &self.resources
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> Option<wgpu::Buffer> {
        if self.attributes.is_empty() {
            return None;
        }

        let padded = self
            .attributes
            .iter()
            .map(|a| a.padded())
            .flatten()
            .collect::<Vec<f32>>();

        Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("material_buffer"),
                contents: bytemuck::cast_slice(&padded),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
        )
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        resources: &GpuResources,
    ) -> wgpu::BindGroup {
        let mut entries = vec![];
        let offset = 1;

        for resource in &self.resources {
            match resource {
                MaterialResource::Texture2D(texture) => {
                    let view = resources
                        .texture_view(texture)
                        .unwrap_or(&resources.defaults.texture_2d_view);
                    entries.push(wgpu::BindGroupEntry {
                        binding: offset + entries.len() as u32,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                MaterialResource::TextureCube(texture) => {
                    let view = resources
                        .texture_view(texture)
                        .unwrap_or(&resources.defaults.texture_cube_view);
                    entries.push(wgpu::BindGroupEntry {
                        binding: offset + entries.len() as u32,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                MaterialResource::Sampler(sampler) => {
                    let sampler = resources
                        .sampler(sampler)
                        .unwrap_or(&resources.defaults.sampler);
                    entries.push(wgpu::BindGroupEntry {
                        binding: offset + entries.len() as u32,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    });
                }
            }
        }

        let buffer = self.create_buffer(device);

        if let Some(buffer) = buffer {
            entries.insert(
                0,
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
            );

            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material_bind_group"),
                layout: &self.create_bind_group_layout(device),
                entries: &entries,
            })
        } else {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material_bind_group"),
                layout: &self.create_bind_group_layout(device),
                entries: &entries,
            })
        }
    }

    pub fn create_bind_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let mut entries = vec![];

        if !self.attributes.is_empty() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }

        for resource in &self.resources {
            match resource {
                MaterialResource::Texture2D(_) => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: entries.len() as u32,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    });
                }
                MaterialResource::TextureCube(_) => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                }),
                MaterialResource::Sampler(_) => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }),
            }
        }

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_bind_group_layout"),
            entries: &entries,
        })
    }
}
