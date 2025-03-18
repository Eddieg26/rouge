use super::{
    AtomicId, Id,
    buffer::Buffer,
    extract::ExtractError,
    texture::{GpuTexture, Sampler},
};
use crate::device::RenderDevice;
use ecs::system::{ArgItem, SystemArg};
use encase::ShaderType;
use std::{error::Error, num::NonZero, sync::Arc};
use wgpu::SamplerBindingType;

#[derive(Clone, Debug, PartialEq)]
pub struct BindGroupLayout(Arc<wgpu::BindGroupLayout>);
impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(layout: wgpu::BindGroupLayout) -> Self {
        Self(Arc::new(layout))
    }
}

impl std::ops::Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<wgpu::BindGroupLayout> for BindGroupLayout {
    fn as_ref(&self) -> &wgpu::BindGroupLayout {
        &self.0
    }
}

pub type BindGroupId = AtomicId<BindGroup>;

#[derive(Clone, Debug, PartialEq)]
pub struct BindGroup {
    pub id: BindGroupId,
    bind_group: Arc<wgpu::BindGroup>,
}

impl From<wgpu::BindGroup> for BindGroup {
    fn from(group: wgpu::BindGroup) -> Self {
        Self {
            id: BindGroupId::new(),
            bind_group: Arc::new(group),
        }
    }
}

impl std::ops::Deref for BindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

impl AsRef<wgpu::BindGroup> for BindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

pub struct BindGroupLayoutBuilder {
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn buffer(
        &mut self,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
        dynamic: bool,
        size: Option<wgpu::BufferSize>,
        count: Option<NonZero<u32>>,
    ) -> &mut Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.entries.len() as u32,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: dynamic,
                min_binding_size: size,
            },
            count,
        });
        self
    }

    pub fn uniform(
        &mut self,
        visibility: wgpu::ShaderStages,
        dynamic: bool,
        size: Option<wgpu::BufferSize>,
        count: Option<NonZero<u32>>,
    ) -> &mut Self {
        self.buffer(
            visibility,
            wgpu::BufferBindingType::Uniform,
            dynamic,
            size,
            count,
        )
    }

    pub fn storage(
        &mut self,
        visibility: wgpu::ShaderStages,
        dynamic: bool,
        size: Option<wgpu::BufferSize>,
        count: Option<NonZero<u32>>,
    ) -> &mut Self {
        self.buffer(
            visibility,
            wgpu::BufferBindingType::Storage { read_only: false },
            dynamic,
            size,
            count,
        )
    }

    pub fn texture(
        &mut self,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
    ) -> &mut Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.entries.len() as u32,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        self
    }

    pub fn sampler(&mut self, visibility: wgpu::ShaderStages) -> &mut Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding: self.entries.len() as u32,
            visibility,
            ty: wgpu::BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        });
        self
    }

    pub fn build(&self, device: &RenderDevice) -> BindGroupLayout {
        BindGroupLayout::from(
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &self.entries,
                label: None,
            }),
        )
    }
}

pub struct BindGroupBuilder<'a> {
    layout: &'a BindGroupLayout,
    entries: Vec<wgpu::BindGroupEntry<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(layout: &'a BindGroupLayout) -> Self {
        Self {
            layout,
            entries: Vec::new(),
        }
    }

    pub fn buffer(
        &mut self,
        binding: u32,
        buffer: &'a Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: buffer.as_ref(),
                offset,
                size,
            }),
        });
        self
    }

    pub fn uniform(
        &mut self,
        binding: u32,
        buffer: &'a Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.buffer(binding, buffer, offset, size)
    }

    pub fn storage(
        &mut self,
        binding: u32,
        buffer: &'a Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.buffer(binding, buffer, offset, size)
    }

    pub fn texture_view(&mut self, binding: u32, view: &'a wgpu::TextureView) -> &mut Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
        self
    }

    pub fn sampler(&mut self, binding: u32, sampler: &'a wgpu::Sampler) -> &mut Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        self
    }

    pub fn build(&self, device: &RenderDevice) -> BindGroup {
        BindGroup::from(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: self.layout,
            entries: &self.entries,
            label: None,
        }))
    }
}

#[derive(Debug, Clone)]
pub enum CreateBindGroupError {
    Error(Arc<dyn Error + Send + Sync + 'static>),
    InvalidLayout,
    MissingTexture { id: Id<GpuTexture> },
    MissingSampler { id: Id<Sampler> },
    MissingBuffer,
}

impl CreateBindGroupError {
    pub fn from_error<E: Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Error(Arc::new(error))
    }
}

impl std::fmt::Display for CreateBindGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(error) => write!(f, "{}", error),
            Self::InvalidLayout => write!(f, "Invalid bind group layout"),
            Self::MissingTexture { id } => write!(f, "Missing texture: {:?}", id),
            Self::MissingSampler { id } => write!(f, "Missing sampler: {:?}", id),
            Self::MissingBuffer => write!(f, "Missing buffer"),
        }
    }
}

impl Error for CreateBindGroupError {}

impl Into<ExtractError> for CreateBindGroupError {
    fn into(self) -> ExtractError {
        match self {
            CreateBindGroupError::Error(error) => ExtractError::Error(error),
            CreateBindGroupError::InvalidLayout => {
                ExtractError::from_error(CreateBindGroupError::InvalidLayout)
            }
            CreateBindGroupError::MissingTexture { .. } => ExtractError::MissingDependency,
            CreateBindGroupError::MissingSampler { .. } => ExtractError::MissingDependency,
            CreateBindGroupError::MissingBuffer => ExtractError::MissingDependency,
        }
    }
}

pub trait CreateBindGroup {
    type Arg: SystemArg + 'static;

    fn label() -> Option<&'static str> {
        None
    }

    fn create_bind_group(
        &self,
        device: &RenderDevice,
        layout: &BindGroupLayout,
        arg: &ArgItem<Self::Arg>,
    ) -> Result<BindGroup, CreateBindGroupError>;
    fn create_bind_group_layout(device: &RenderDevice) -> BindGroupLayout;
}

pub trait IntoBufferData<T: ShaderType> {
    fn into_buffer_data(&self) -> T;
}

impl<T: ShaderType, I> IntoBufferData<T> for I
where
    for<'a> &'a I: Into<T>,
{
    #[inline]
    fn into_buffer_data(&self) -> T {
        self.into()
    }
}

pub trait IntoBindGroupData<T: Send + Sync + 'static> {
    fn into_bind_group_data(&self) -> T;
}

impl<T: Send + Sync + 'static, I> IntoBindGroupData<T> for I
where
    for<'a> &'a I: Into<T>,
{
    #[inline]
    fn into_bind_group_data(&self) -> T {
        self.into()
    }
}
