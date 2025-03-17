use super::AtomicId;
use crate::device::RenderDevice;
use std::{num::NonZero, sync::Arc};
use wgpu::SamplerBindingType;

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
        buffer: &'a wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset,
                size,
            }),
        });
        self
    }

    pub fn uniform(
        &mut self,
        binding: u32,
        offset: wgpu::BufferAddress,
        buffer: &'a wgpu::Buffer,
        size: Option<wgpu::BufferSize>,
    ) -> &mut Self {
        self.buffer(binding, buffer, offset, size)
    }

    pub fn storage(
        &mut self,
        binding: u32,
        offset: wgpu::BufferAddress,
        buffer: &'a wgpu::Buffer,
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
