use super::{sampler::Sampler, texture::TextureDimension, AtomicId, Id, RenderTexture};
use crate::{
    wgpu::{
        BindGroupEntry, BindGroupLayoutEntry, BindingType, BufferBindingType, SamplerBindingType,
        ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
    },
    ExtractError, RenderDevice,
};
use ecs::system::{ArgItem, SystemArg};
use encase::ShaderType;
use std::{error::Error, num::NonZeroU32, sync::Arc};

pub type BindGroupId = AtomicId<BindGroup>;
pub type BindGroupLayoutId = AtomicId<BindGroupLayout>;

pub struct BindGroupLayoutBuilder {
    entries: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_entry(mut self, entry: BindGroupLayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn with_buffer(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        buffer: BufferBindingType,
        min_binding_size: Option<wgpu::BufferSize>,
        array_size: Option<NonZeroU32>,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: buffer,
                has_dynamic_offset: false,
                min_binding_size,
            },
            count: array_size,
        });
        self
    }

    pub fn with_uniform_buffer(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        dynamic: bool,
        min_binding_size: Option<wgpu::BufferSize>,
        array_size: Option<NonZeroU32>,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size,
            },
            count: array_size,
        });
        self
    }

    pub fn with_storage_buffer(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        dynamic: bool,
        read_only: bool,
        min_binding_size: Option<wgpu::BufferSize>,
        count: Option<NonZeroU32>,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only },
                has_dynamic_offset: dynamic,
                min_binding_size,
            },
            count,
        });
        self
    }

    pub fn with_texture(
        mut self,
        binding: u32,
        texture: TextureSampleType,
        dimension: TextureDimension,
        visibility: ShaderStages,
        multisampled: bool,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Texture {
                sample_type: texture,
                view_dimension: dimension.into(),
                multisampled,
            },
            count: None,
        });
        self
    }

    pub fn with_storage_texture(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        access: StorageTextureAccess,
        format: TextureFormat,
        dimension: TextureDimension,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::StorageTexture {
                access,
                format: format.into(),
                view_dimension: dimension.into(),
            },
            count: None,
        });
        self
    }

    pub fn with_sampler(
        mut self,
        binding: u32,
        sampler: SamplerBindingType,
        visibility: ShaderStages,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Sampler(sampler),
            count: None,
        });
        self
    }

    pub fn build(self, device: &RenderDevice) -> BindGroupLayout {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &self.entries,
        });

        BindGroupLayout {
            id: BindGroupLayoutId::new(),
            layout: Arc::new(layout),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    layout: Arc<wgpu::BindGroupLayout>,
}

impl BindGroupLayout {
    pub fn id(&self) -> BindGroupLayoutId {
        self.id
    }

    pub fn inner(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
}

impl std::ops::Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl From<wgpu::BindGroupLayout> for BindGroupLayout {
    fn from(layout: wgpu::BindGroupLayout) -> Self {
        Self {
            id: BindGroupLayoutId::new(),
            layout: Arc::new(layout),
        }
    }
}

pub struct BindGroupEntries<'a> {
    entries: Vec<BindGroupEntry<'a>>,
}

impl<'a> BindGroupEntries<'a> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_entry(mut self, entry: BindGroupEntry<'a>) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn add_buffer(
        &mut self,
        binding: u32,
        buffer: &'a wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) {
        self.entries.push(BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset,
                size,
            }),
        });
    }

    pub fn add_texture(&mut self, binding: u32, view: &'a wgpu::TextureView) {
        self.entries.push(BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
    }

    pub fn add_sampler(&mut self, binding: u32, sampler: &'a wgpu::Sampler) {
        self.entries.push(BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }

    pub fn entries(&self) -> &[BindGroupEntry] {
        &self.entries
    }
}

impl<'a> std::ops::Deref for BindGroupEntries<'a> {
    type Target = [BindGroupEntry<'a>];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

#[derive(Debug, Clone)]
pub struct BindGroup<D: Send + Sync + 'static = ()> {
    id: BindGroupId,
    binding: Arc<wgpu::BindGroup>,
    data: D,
}

impl<D: Send + Sync + 'static> BindGroup<D> {
    pub fn create(
        device: &RenderDevice,
        layout: &BindGroupLayout,
        entries: &[BindGroupEntry],
        data: D,
    ) -> Self {
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries,
        });

        Self {
            id: BindGroupId::new(),
            binding: Arc::new(binding),
            data,
        }
    }

    pub fn id(&self) -> BindGroupId {
        self.id
    }

    #[inline]
    pub fn inner(&self) -> &wgpu::BindGroup {
        &self.binding
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }
}

impl From<wgpu::BindGroup> for BindGroup<()> {
    fn from(binding: wgpu::BindGroup) -> Self {
        Self {
            id: BindGroupId::new(),
            binding: Arc::new(binding),
            data: (),
        }
    }
}

impl<D: Send + Sync + Clone + 'static> std::ops::Deref for BindGroup<D> {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

#[derive(Debug, Clone)]
pub enum CreateBindGroupError {
    Error(Arc<dyn Error + Send + Sync + 'static>),
    InvalidLayout,
    MissingTexture { id: Id<RenderTexture> },
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

impl std::error::Error for CreateBindGroupError {}

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
    type Data: Send + Sync + 'static;

    fn label() -> Option<&'static str> {
        None
    }

    fn bind_group(
        &self,
        device: &RenderDevice,
        layout: &BindGroupLayout,
        arg: &ArgItem<Self::Arg>,
    ) -> Result<BindGroup<Self::Data>, CreateBindGroupError>;
    fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout;
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
