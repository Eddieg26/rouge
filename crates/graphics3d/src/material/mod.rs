use asset::{io::cache::LoadPath, Asset, AssetId};
use ecs::core::{resource::Resource, IndexMap, Type};
use graphics::{
    resource::{
        binding::{BindGroup, BindGroupLayout, CreateBindGroup},
        pipeline::RenderPipeline,
        shader::meta::ShaderMeta,
    },
    wgpu::PrimitiveState,
    RenderAsset,
};
use std::collections::HashSet;

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

pub trait Surface: Send + Sync + 'static {
    fn depth_write() -> DepthWrite {
        DepthWrite::On
    }
    fn primitive() -> PrimitiveState;
    fn shader() -> impl Into<LoadPath>;
}

pub trait Material: Asset + CreateBindGroup<Data = ()> + 'static {
    type Surface: Surface;

    fn mode() -> BlendMode;
    fn model() -> ShaderModel;
    fn shader() -> impl Into<LoadPath>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialType {
    pub surface: Type,
    pub material: Type,
}

impl MaterialType {
    pub fn of<M: Material>() -> Self {
        let surface = Type::of::<M::Surface>();
        let material = Type::of::<M>();
        Self { surface, material }
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

pub struct MaterialResources {
    layouts: IndexMap<MaterialType, BindGroupLayout>,
    pipelines: IndexMap<MaterialType, RenderPipeline>,
    dependencies: IndexMap<MaterialType, HashSet<AssetId>>,
}

impl MaterialResources {
    pub fn new() -> Self {
        Self {
            layouts: IndexMap::new(),
            pipelines: IndexMap::new(),
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

    pub fn add_pipeline(&mut self, ty: MaterialType, pipeline: RenderPipeline) {
        self.pipelines.insert(ty, pipeline);
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

impl Resource for MaterialResources {}

mod t {
    use super::{Material, Surface};
    use asset::{io::cache::LoadPath, Asset};
    use graphics::{
        encase::ShaderType,
        resource::{
            BindGroup, BindGroupLayout, BuiltinValue, Id, IntoBindGroupData, IntoBufferData,
            IntoOptionalId, Mesh, RenderTexture, ShaderAttribute, ShaderMeta, ShaderValue,
        },
        wgpu::PrimitiveState,
        Color, CreateBindGroup, RenderDevice,
    };

    impl Surface for Mesh {
        fn primitive() -> PrimitiveState {
            PrimitiveState::default()
        }

        fn shader() -> impl Into<LoadPath> {
            ""
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Asset, CreateBindGroup)]
    pub struct Standard<S: Surface> {
        albedo_color: Color,
        other_color: Color,
        #[texture(1)]
        #[sampler(1)]
        albedo_texture: Option<Id<RenderTexture>>,
        _marker: std::marker::PhantomData<S>,
    }

    impl<S: Surface> Material for Standard<S> {
        type Surface = S;

        fn mode() -> super::BlendMode {
            todo!()
        }

        fn model() -> super::ShaderModel {
            todo!()
        }

        fn shader() -> impl Into<LoadPath> {
            ""
        }
    }

    impl<S: Surface> IntoBufferData<StandardBufferData> for Standard<S> {
        fn into_buffer_data(&self) -> StandardBufferData {
            StandardBufferData {
                albedo_color: self.albedo_color,
                other_color: self.other_color,
            }
        }
    }

    impl<S: Surface> IntoBindGroupData<StandardBufferData> for Standard<S> {
        fn into_bind_group_data(&self) -> StandardBufferData {
            StandardBufferData {
                albedo_color: self.albedo_color,
                other_color: self.other_color,
            }
        }
    }

    #[derive(ShaderType)]
    pub struct StandardBufferData {
        albedo_color: Color,
        other_color: Color,
    }

    // impl<S: Surface> CreateBindGroup for Standard<S> {
    //     type Arg = ();

    //     type Data = ();

    //     fn bind_group(
    //         &self,
    //         device: &RenderDevice,
    //         layout: &BindGroupLayout,
    //         arg: &ecs::system::ArgItem<Self::Arg>,
    //     ) -> Result<BindGroup<Self::Data>, CreateBindGroupError> {
    //         todo!()
    //     }

    //     fn bind_group_layout(device: &RenderDevice) -> BindGroupLayout {
    //         todo!()
    //     }
    // }
}
