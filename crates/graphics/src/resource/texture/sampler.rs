use super::{FilterMode, WrapMode};
use crate::{
    resource::Id,
    wgpu::{CompareFunction, SamplerBorderColor},
    RenderAsset,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SamplerDesc<'a> {
    pub label: Option<&'a str>,
    pub wrap_mode: WrapMode,
    pub filter_mode: FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<CompareFunction>,
    pub anisotropy_clamp: u16,
    pub border_color: Option<SamplerBorderColor>,
}

impl Default for SamplerDesc<'_> {
    fn default() -> Self {
        Self {
            label: None,
            wrap_mode: WrapMode::ClampToEdge,
            filter_mode: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        }
    }
}

pub struct Sampler(wgpu::Sampler);

impl Sampler {
    pub fn create(device: &wgpu::Device, desc: &SamplerDesc) -> Self {
        let address_mode = desc.wrap_mode.into();
        let filter_mode = desc.filter_mode.into();

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
            lod_min_clamp: desc.lod_min_clamp,
            lod_max_clamp: desc.lod_max_clamp,
            compare: desc.compare,
            anisotropy_clamp: desc.anisotropy_clamp,
            border_color: desc.border_color,
        });

        Self(sampler)
    }

    pub fn inner(&self) -> &wgpu::Sampler {
        &self.0
    }
}

impl std::ops::Deref for Sampler {
    type Target = wgpu::Sampler;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<wgpu::Sampler> for Sampler {
    fn from(sampler: wgpu::Sampler) -> Self {
        Self(sampler)
    }
}

impl RenderAsset for Sampler {
    type Id = Id<Sampler>;
}
