use super::buffer::{AccessMode, BufferLayout, ShaderBuffer};
use rouge_core::ResourceId;
use std::{
    hash::{Hash, Hasher},
    num::{NonZeroU32, NonZeroU64},
};

pub trait IntoBindGroupLayout: 'static {
    fn into_bind_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderBinding {
    UniformBuffer {
        layout: BufferLayout,
        count: Option<NonZeroU32>,
    },
    StorageBuffer {
        layout: BufferLayout,
        access: AccessMode,
        count: Option<NonZeroU32>,
    },
    Texture2D,
    TextureCube,
    Sampler,
}

impl std::fmt::Display for ShaderBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderBinding::UniformBuffer { layout, count } => {
                write!(
                    f,
                    "uniform_buffer({}, {})",
                    layout,
                    count.unwrap_or(NonZeroU32::new(1).unwrap())
                )
            }
            ShaderBinding::StorageBuffer {
                layout,
                access,
                count,
            } => write!(
                f,
                "storage_buffer({}, {}, {})",
                layout,
                access,
                count.unwrap_or(NonZeroU32::new(1).unwrap())
            ),
            ShaderBinding::Texture2D => write!(f, "texture_2d"),
            ShaderBinding::TextureCube => write!(f, "texture_cube"),
            ShaderBinding::Sampler => write!(f, "sampler"),
        }
    }
}

pub struct ShaderBindGroup {
    group: u32,
    bindings: Vec<ShaderBinding>,
}

impl ShaderBindGroup {
    pub fn new(group: u32, bindings: Vec<ShaderBinding>) -> Self {
        Self { group, bindings }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn bindings(&self) -> &[ShaderBinding] {
        &self.bindings
    }
}

impl IntoBindGroupLayout for ShaderBindGroup {
    fn into_bind_group_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let mut entries = vec![];

        for binding in &self.bindings {
            match binding {
                ShaderBinding::UniformBuffer { layout, count } => {
                    entries.push(wgpu::BindGroupLayoutEntry {
                        binding: entries.len() as u32,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(NonZeroU64::new(layout.size() as u64).unwrap()),
                        },
                        count: *count,
                    })
                }
                ShaderBinding::StorageBuffer {
                    layout,
                    access,
                    count,
                } => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: *access == AccessMode::Read,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZeroU64::new(layout.size() as u64).unwrap()),
                    },
                    count: *count,
                }),
                ShaderBinding::Texture2D => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }),
                ShaderBinding::TextureCube => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                }),
                ShaderBinding::Sampler => entries.push(wgpu::BindGroupLayoutEntry {
                    binding: entries.len() as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }),
            }
        }

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        })
    }
}

impl Into<ResourceId> for ShaderBindGroup {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.bindings.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}

impl std::fmt::Display for ShaderBindGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "group:{}\n\tresources:\n{}\n",
            self.group,
            self.bindings
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join("\n\t")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub enum ShaderResource {
    UniformBuffer(ShaderBuffer),
    StorageBuffer(ShaderBuffer),
    Texture2D(ResourceId),
    TextureCube(ResourceId),
    Sampler(ResourceId),
}

impl std::fmt::Display for ShaderResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderResource::Texture2D(id) => write!(f, "texture_2d({})", id),
            ShaderResource::TextureCube(id) => write!(f, "texture_cube({})", id),
            ShaderResource::Sampler(id) => write!(f, "sampler({})", id),
            ShaderResource::UniformBuffer(buffer) => write!(f, "uniform_buffer({})", buffer),
            ShaderResource::StorageBuffer(buffer) => write!(f, "storage_buffer({})", buffer),
        }
    }
}

pub struct ShaderResourceGroup {
    group: u32,
    resources: Vec<ShaderResource>,
}

impl ShaderResourceGroup {
    pub fn new(group: u32, resources: Vec<ShaderResource>) -> Self {
        Self { group, resources }
    }

    pub fn group(&self) -> u32 {
        self.group
    }

    pub fn resources(&self) -> &[ShaderResource] {
        &self.resources
    }
}

impl std::fmt::Display for ShaderResourceGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[group:{}\n resources:{}]",
            self.group,
            self.resources
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl Into<ResourceId> for ShaderResourceGroup {
    fn into(self) -> ResourceId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.resources.hash(&mut hasher);
        ResourceId::new(hasher.finish())
    }
}
