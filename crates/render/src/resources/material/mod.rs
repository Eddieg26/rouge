use super::{RenderAssetExtractor, RenderResource};
use crate::{
    device::RenderDevice,
    resources::{
        binding::{AsBinding, BindGroup, BindGroupLayout},
        extract::RenderAsset,
        shader::ShaderPath,
    },
};
use asset::{AssetRef, asset::Asset};
use ecs::{Component, Resource, system::unlifetime::ReadRes};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

impl Into<wgpu::BlendState> for BlendMode {
    fn into(self) -> wgpu::BlendState {
        match self {
            BlendMode::Opaque => wgpu::BlendState::REPLACE,
            BlendMode::Transparent => wgpu::BlendState::ALPHA_BLENDING,
        }
    }
}

pub trait Material: Asset + AsBinding + Clone + Sized {
    fn mode() -> BlendMode;
    fn shader() -> impl Into<ShaderPath>;
}

pub struct MaterialLayout<M: Material> {
    layout: BindGroupLayout,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> Clone for MaterialLayout<M> {
    fn clone(&self) -> Self {
        Self {
            layout: self.layout.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: Material> MaterialLayout<M> {
    pub fn new(device: &RenderDevice) -> Self {
        let layout = M::create_bind_group_layout(device);
        Self {
            layout,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: Material> std::ops::Deref for MaterialLayout<M> {
    type Target = BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl<M: Material> AsRef<BindGroupLayout> for MaterialLayout<M> {
    fn as_ref(&self) -> &BindGroupLayout {
        &self.layout
    }
}

impl<M: Material> Resource for MaterialLayout<M> {}
impl<M: Material> RenderResource for MaterialLayout<M> {
    type Arg = ReadRes<RenderDevice>;

    fn extract(device: ecs::system::ArgItem<Self::Arg>) -> Result<Self, super::ExtractError<()>> {
        Ok(Self::new(&device))
    }
}

pub struct MaterialBinding<M: Material> {
    bind_group: BindGroup,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> std::ops::Deref for MaterialBinding<M> {
    type Target = BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.bind_group
    }
}

impl<M: Material> RenderAsset for MaterialBinding<M> {}

impl<M: Material> RenderAssetExtractor for M {
    type RenderAsset = MaterialBinding<M>;

    type Arg = (
        ReadRes<RenderDevice>,
        Option<ReadRes<MaterialLayout<M>>>,
        M::Arg,
    );

    fn extract(
        asset: Self,
        arg: &mut ecs::prelude::ArgItem<Self::Arg>,
    ) -> Result<Self::RenderAsset, super::ExtractError<Self>> {
        let (device, layout, arg) = arg;
        let layout = match layout.as_ref() {
            Some(layout) => layout,
            None => return Err(super::ExtractError::Retry(asset)),
        };

        let binding = asset
            .create_bind_group(device, &layout, arg)
            .map_err(|_| super::ExtractError::Retry(asset))?;

        Ok(MaterialBinding {
            bind_group: binding,
            _marker: std::marker::PhantomData,
        })
    }
}

pub struct MaterialRef<M: Material>(AssetRef<M>);
impl<M: Material> MaterialRef<M> {
    pub fn new(asset: impl Into<AssetRef<M>>) -> Self {
        Self(asset.into())
    }

    pub fn set(&mut self, asset: AssetRef<M>) {
        self.0 = asset;
    }
}

impl<M: Material> From<AssetRef<M>> for MaterialRef<M> {
    fn from(asset: AssetRef<M>) -> Self {
        Self::new(asset)
    }
}

impl<M: Material> From<MaterialRef<M>> for AssetRef<M> {
    fn from(asset: MaterialRef<M>) -> Self {
        asset.0
    }
}

impl<M: Material> std::ops::Deref for MaterialRef<M> {
    type Target = AssetRef<M>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<M: Material> AsRef<AssetRef<M>> for MaterialRef<M> {
    fn as_ref(&self) -> &AssetRef<M> {
        &self.0
    }
}

impl<M: Material> Copy for MaterialRef<M> {}
impl<M: Material> Clone for MaterialRef<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M: Material> Component for MaterialRef<M> {}
