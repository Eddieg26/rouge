use asset::{io::cache::LoadPath, Asset, AssetId};
use ecs::core::{resource::Resource, IndexMap, Type};
use graphics::{
    core::RenderAsset,
    resource::{
        binding::{BindGroup, BindGroupLayout, CreateBindGroup},
        pipeline::PrimitiveState,
        shader::meta::ShaderMeta,
    },
};
use std::{collections::HashSet, num::NonZeroU32};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait Surface: 'static {
    fn depth_write() -> DepthWrite;
    fn primitive() -> PrimitiveState;
    fn shader() -> impl Into<LoadPath>;
    fn meta() -> ShaderMeta;
    fn instances() -> Option<NonZeroU32> {
        None
    }
}

pub trait Material: Asset + Clone + Sized + CreateBindGroup + 'static {
    type Surface: Surface;

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

pub struct MaterialMeta {
    pub model: ShaderModel,
    pub mode: BlendMode,
    pub meta: ShaderMeta,
    pub shader: LoadPath,
}

impl MaterialMeta {
    pub fn new<M: Material>() -> Self {
        Self {
            model: M::model(),
            mode: M::mode(),
            meta: M::meta(),
            shader: M::shader().into(),
        }
    }
}

pub struct MaterialRegistry {
    materials: IndexMap<MaterialType, MaterialMeta>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self {
            materials: IndexMap::new(),
        }
    }

    pub fn register<M: Material>(&mut self) {
        self.materials.insert(
            MaterialType::of::<M>(),
            MaterialMeta {
                model: M::model(),
                mode: M::mode(),
                meta: M::meta(),
                shader: M::shader().into(),
            },
        );
    }

    pub fn get(&self, ty: MaterialType) -> Option<&MaterialMeta> {
        self.materials.get(&ty)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MaterialType, &MaterialMeta)> {
        self.materials.iter()
    }
}

impl Resource for MaterialRegistry {}

pub struct MaterialLayouts {
    layouts: IndexMap<MaterialType, BindGroupLayout>,
    dependencies: IndexMap<MaterialType, HashSet<AssetId>>,
}

impl MaterialLayouts {
    pub fn new() -> Self {
        Self {
            layouts: IndexMap::new(),
            dependencies: IndexMap::new(),
        }
    }

    pub fn get(&self, ty: &MaterialType) -> Option<&BindGroupLayout> {
        self.layouts.get(ty)
    }

    pub fn has(&self, ty: &MaterialType) -> bool {
        self.layouts.contains_key(ty)
    }

    pub fn add_layout(&mut self, ty: MaterialType, layout: BindGroupLayout) {
        self.layouts.insert(ty, layout);
    }

    pub fn add_dependency(&mut self, ty: MaterialType, id: AssetId) {
        self.dependencies.entry(ty).or_default().insert(id);
    }

    pub fn remove_dependency(&mut self, ty: MaterialType, id: AssetId) {
        let removed = match self.dependencies.get_mut(&ty) {
            Some(dependencies) => {
                dependencies.remove(&id);
                dependencies.is_empty()
            }
            None => false,
        };

        if removed {
            self.layouts.shift_remove(&ty);
        }
    }

    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    pub fn clear(&mut self) {
        self.layouts.clear();
    }
}

impl Resource for MaterialLayouts {}
