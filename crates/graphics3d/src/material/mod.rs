use asset::{io::cache::LoadPath, Asset, AssetId};
use ecs::core::{IndexMap, Type};
use graphics::{
    core::RenderAsset,
    resources::{
        binding::{BindGroup, BindGroupLayout, CreateBindGroup},
        shader::meta::ShaderMeta,
    },
};

pub mod pipeline;
pub mod registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderModel {
    Unlit,
    Lit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Transparent,
}

pub trait Material: Asset + Clone + Sized + CreateBindGroup + 'static {
    fn mode() -> BlendMode;
    fn model() -> ShaderModel;
    fn meta() -> ShaderMeta;
    fn shader() -> impl Into<LoadPath>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType(Type);

impl MaterialType {
    pub fn of<M: Material>() -> Self {
        Self(Type::of::<M>())
    }
}

#[derive(Debug, Clone)]
pub struct MaterialInstance {
    pub binding: BindGroup,
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub ty: MaterialType,
}

impl RenderAsset for MaterialInstance {
    type Id = AssetId;
}

pub struct MaterialLayout {
    pub layout: BindGroupLayout,
    pub ref_count: usize,
}

pub struct MaterialLayouts {
    layouts: IndexMap<MaterialType, MaterialLayout>,
}

impl MaterialLayouts {
    pub fn new() -> Self {
        Self {
            layouts: IndexMap::new(),
        }
    }

    pub fn get(&self, ty: &MaterialType) -> Option<&BindGroupLayout> {
        self.layouts.get(ty).map(|layout| &layout.layout)
    }

    pub fn has(&self, ty: &MaterialType) -> bool {
        self.layouts.contains_key(ty)
    }

    pub fn add(&mut self, ty: MaterialType, layout: BindGroupLayout) {
        self.layouts.insert(
            ty,
            MaterialLayout {
                layout,
                ref_count: 1,
            },
        );
    }

    pub fn reference(&mut self, ty: &MaterialType) {
        if let Some(layout) = self.layouts.get_mut(ty) {
            layout.ref_count += 1;
        }
    }

    pub fn remove(&mut self, ty: &MaterialType) -> Option<BindGroupLayout> {
        let remove = match self.layouts.get_mut(ty) {
            Some(layout) => {
                layout.ref_count -= 1;
                layout.ref_count == 0
            }
            None => false,
        };

        match remove {
            true => self.layouts.shift_remove(ty).map(|layout| layout.layout),
            false => None,
        }
    }

    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    pub fn clear(&mut self) {
        self.layouts.clear();
    }
}
