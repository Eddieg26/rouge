use super::{
    texture::{TextureDimension, TextureFormat},
    AtomicId,
};
use crate::core::device::RenderDevice;
use ecs::system::{ArgItem, SystemArg};
use std::sync::Arc;
use wgpu::BindGroupLayoutEntry;

pub type BindGroupId = AtomicId<BindGroup>;
pub type BindGroupLayoutId = AtomicId<BindGroupLayout>;

pub use wgpu::{
    BindGroupEntry, BindingType, BufferBindingType, SamplerBindingType, ShaderStages,
    StorageTextureAccess, TextureSampleType,
};

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
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: buffer,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    pub fn with_texture(
        mut self,
        binding: u32,
        visibility: ShaderStages,
        texture: TextureSampleType,
        dimension: TextureDimension,
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
        visibility: ShaderStages,
        sampler: SamplerBindingType,
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
    pub fn new(layout: wgpu::BindGroupLayout) -> Self {
        Self {
            id: BindGroupLayoutId::new(),
            layout: Arc::new(layout),
        }
    }

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
    ) -> Option<BindGroup<Self::Data>>;
    fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout;
}
