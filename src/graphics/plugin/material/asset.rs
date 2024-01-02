use crate::{
    asset::{Asset, AssetId, AssetLoader, Assets},
    ecs::{
        resource::{ResourceId, ResourceType},
        Resource, World,
    },
    graphics::{
        core::{device::RenderDevice, BlendMode, ShaderModel},
        resources::{
            bind_group::BindGroups,
            buffer::Buffers,
            pipeline::Pipelines,
            shader::{
                layout::{
                    BufferInfo, BufferLayout, BufferType, ShaderBinding, ShaderBindings,
                    ShaderInput, ShaderResource, ShaderResources, ShaderVariable,
                },
                Shader,
            },
            texture::TextureResources,
            ShaderId, TextureId,
        },
    },
};
use std::{any::TypeId, collections::HashMap};

pub type MaterialId = ResourceId;

pub trait Material: 'static + Asset + serde::Serialize + serde::de::DeserializeOwned {
    fn fragment_shader(&self) -> ShaderId;

    fn model() -> ShaderModel {
        ShaderModel::Unlit
    }

    fn blend_mode() -> BlendMode {
        BlendMode::Opaque
    }

    fn layout() -> ShaderBindings {
        ShaderBindings::new()
    }

    fn resources(&self) -> ShaderResources {
        ShaderResources::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnlitTexture {
    texture: TextureId,
}

impl UnlitTexture {
    pub fn new(texture: TextureId) -> UnlitTexture {
        UnlitTexture { texture }
    }

    pub fn texture(&self) -> TextureId {
        self.texture
    }

    pub fn set_texture(&mut self, texture: TextureId) {
        self.texture = texture;
    }
}

impl Material for UnlitTexture {
    fn fragment_shader(&self) -> ShaderId {
        ShaderId::from("unlit_texture")
    }

    fn layout() -> ShaderBindings {
        vec![ShaderBinding::Texture2D].into()
    }

    fn resources(&self) -> ShaderResources {
        vec![ShaderResource::Texture2D(self.texture)].into()
    }
}

impl Asset for UnlitTexture {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnLitColor {
    color: [f32; 4],
}

impl UnLitColor {
    pub fn new(color: [f32; 4]) -> UnLitColor {
        UnLitColor { color }
    }

    pub fn color(&self) -> &[f32; 4] {
        &self.color
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }
}

impl Material for UnLitColor {
    fn fragment_shader(&self) -> ShaderId {
        ShaderId::from("unlit_color")
    }

    fn layout() -> ShaderBindings {
        vec![ShaderBinding::UniformBuffer {
            layout: BufferLayout::new(&[ShaderVariable::Vec4]),
            count: None,
        }]
        .into()
    }

    fn resources(&self) -> ShaderResources {
        vec![ShaderResource::Buffer(
            BufferInfo::new(BufferType::Uniform).add_input(ShaderInput::Vec4(self.color.into())),
        )]
        .into()
    }
}

impl Asset for UnLitColor {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialObj {
    ty: ResourceType,
    fragment_shader: ShaderId,
    blend_mode: BlendMode,
    model: ShaderModel,
    resources: ShaderResources,
}

impl MaterialObj {
    pub fn new<M: Material>(material: &M) -> MaterialObj {
        MaterialObj {
            ty: TypeId::of::<M>().into(),
            fragment_shader: material.fragment_shader(),
            blend_mode: M::blend_mode(),
            model: M::model(),
            resources: material.resources(),
        }
    }

    pub fn ty(&self) -> ResourceType {
        self.ty
    }

    pub fn fragment_shader(&self) -> ShaderId {
        self.fragment_shader
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn resources(&self) -> &ShaderResources {
        &self.resources
    }
}

pub struct MaterialReflection {
    blend_mode: BlendMode,
    model: ShaderModel,
    layout: ShaderBindings,
    serialize: fn(&ResourceId, &World) -> Option<String>,
    deserialize: fn(&ResourceId, &str, &World) -> Option<MaterialObj>,
}

impl MaterialReflection {
    pub fn new<M: Material>() -> MaterialReflection {
        MaterialReflection {
            blend_mode: M::blend_mode(),
            model: M::model(),
            layout: M::layout(),

            serialize: |id, world| -> Option<String> {
                let assets = world.resource::<Assets<M>>();
                let material = assets.get(id)?;

                Some(toml::to_string(material).ok()?)
            },

            deserialize: |id, data, world| -> Option<MaterialObj> {
                let material = toml::from_str::<M>(data).ok()?;
                let mut assets = world.resource_mut::<Assets<M>>();
                let erased = MaterialObj::new::<M>(&material);
                assets.insert(*id, material);

                Some(erased)
            },
        }
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    pub fn model(&self) -> ShaderModel {
        self.model
    }

    pub fn layout(&self) -> &ShaderBindings {
        &self.layout
    }

    pub fn serialize(&self, id: &ResourceId, world: &World) -> Option<String> {
        (self.serialize)(id, world)
    }

    pub fn deserialize(&self, id: &ResourceId, data: &str, world: &World) -> Option<MaterialObj> {
        (self.deserialize)(id, data, world)
    }
}

pub struct MaterialRegistry {
    registry: HashMap<ResourceType, MaterialReflection>,
}

impl MaterialRegistry {
    pub fn new() -> MaterialRegistry {
        MaterialRegistry {
            registry: HashMap::new(),
        }
    }

    pub fn register<M: Material>(&mut self) {
        let reflection = MaterialReflection::new::<M>();
        self.registry.insert(TypeId::of::<M>().into(), reflection);
    }

    pub fn reflect(&self, ty: ResourceType) -> Option<&MaterialReflection> {
        self.registry.get(&ty)
    }
}

impl Resource for MaterialRegistry {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Asset for MaterialObj {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializedMaterial {
    ty: ResourceType,
    data: String,
}

impl SerializedMaterial {
    pub fn new(ty: ResourceType, data: String) -> SerializedMaterial {
        SerializedMaterial { ty, data }
    }

    pub fn ty(&self) -> ResourceType {
        self.ty
    }

    pub fn data(&self) -> &str {
        &self.data
    }
}

impl AssetLoader for MaterialObj {
    type Asset = MaterialObj;
    type Metadata = ();

    fn extensions() -> &'static [&'static str] {
        &["material"]
    }

    fn load(ctx: crate::asset::LoadContext, _: Self::Metadata) -> Option<Self::Asset> {
        let contents = std::fs::read_to_string(ctx.path()?).ok()?;
        let material = toml::from_str::<SerializedMaterial>(&contents).ok()?;

        let material = {
            let registry = ctx.world().resource::<MaterialRegistry>();
            let reflection = registry.reflect(material.ty())?;
            reflection.deserialize(ctx.id(), material.data(), ctx.world())?
        };

        Some(material)
    }

    fn postprocess<'a>(
        ctx: crate::asset::LoadContext,
        materials: impl Iterator<Item = (&'a AssetId, &'a Self::Asset)>,
    ) {
        let render_device = ctx.resource::<RenderDevice>();
        let shaders = ctx.resource::<Assets<Shader>>();
        let textures = TextureResources::from_world(ctx.world());
        let mut buffers = ctx.resource_mut::<Buffers>();
        let mut bind_groups = ctx.resource_mut::<BindGroups>();
        let mut pipelines = ctx.resource_mut::<Pipelines>();

        for (id, material) in materials {
            let fragment_id = material.fragment_shader();
            let blend_mode = material.blend_mode();

            bind_groups.create_bind_group(
                render_device.inner(),
                &textures,
                &mut buffers,
                id,
                material.resources(),
            );

            pipelines.add_pipelines(
                render_device.inner(),
                fragment_id,
                blend_mode,
                &shaders,
                &bind_groups,
            );
        }
    }
}

impl<M: Material> AssetLoader for M {
    type Asset = M;
    type Metadata = ();

    fn extensions() -> &'static [&'static str] {
        &[]
    }

    fn load(_: crate::asset::LoadContext, _: Self::Metadata) -> Option<Self::Asset> {
        None
    }

    fn unload(ctx: crate::asset::LoadContext, _: &Self::Asset) {
        let mut bind_groups = ctx.resource_mut::<BindGroups>();
        let mut materials = ctx.resource_mut::<Assets<M>>();
        let mut material_objs = ctx.resource_mut::<Assets<MaterialObj>>();

        bind_groups.remove_bind_group(ctx.id());
        materials.remove(ctx.id());
        material_objs.remove(ctx.id());
    }
}

impl<M: Material> Assets<M> {
    pub fn update(&mut self, world: &World, id: &MaterialId, material: M) {
        if let Some(prev_material) = self.insert(*id, material) {
            let recreate_bind_group =
                prev_material
                    .resources()
                    .inner()
                    .iter()
                    .enumerate()
                    .any(|(index, resource)| {
                        let resources = prev_material.resources();
                        let prev = resources.inner().get(index).unwrap();
                        match (prev, resource) {
                            (ShaderResource::Buffer(prev), ShaderResource::Buffer(resource)) => {
                                prev != resource
                            }
                            (
                                ShaderResource::Texture2D(prev),
                                ShaderResource::Texture2D(resource),
                            ) => prev != resource,
                            (
                                ShaderResource::TextureCube(prev),
                                ShaderResource::TextureCube(resource),
                            ) => prev != resource,
                            (ShaderResource::Sampler(prev), ShaderResource::Sampler(resource)) => {
                                prev != resource
                            }
                            _ => true,
                        }
                    });

            if recreate_bind_group {
                self.create_bind_group(world, id)
            }
        } else {
            self.create_bind_group(world, id)
        }
    }

    pub fn create_bind_group(&self, world: &World, id: &MaterialId) {
        if let Some(material) = self.get(id) {
            let render_device = world.resource::<RenderDevice>();
            let textures = TextureResources::from_world(world);
            let mut bind_groups = world.resource_mut::<BindGroups>();
            let mut buffers = world.resource_mut::<Buffers>();

            bind_groups.create_bind_group(
                render_device.inner(),
                &textures,
                &mut buffers,
                id,
                &material.resources(),
            );
        }
    }
}
