use crate::resources::{
    binding::{BindGroup, BindGroupLayout, CreateBindGroup},
    extract::RenderAsset,
    shader::ShaderPath,
};
use asset::asset::Asset;

pub mod draw;

pub enum BlendMode {
    Opaque,
    Transparent,
}

pub trait Material: Asset + CreateBindGroup + Send + Sync + 'static {
    fn mode() -> BlendMode;
    fn shader() -> impl Into<ShaderPath>;
}

pub struct MaterialLayout<M: Material> {
    layout: BindGroupLayout,
    _marker: std::marker::PhantomData<M>,
}

impl<M: Material> std::ops::Deref for MaterialLayout<M> {
    type Target = BindGroupLayout;

    fn deref(&self) -> &Self::Target {
        &self.layout
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
