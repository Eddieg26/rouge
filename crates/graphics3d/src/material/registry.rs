use super::{
    pipeline::{DepthWrite, MaterialPipeline, MaterialPipelineKey},
    BlendMode, Material, MaterialType, ShaderModel,
};
use asset::io::cache::LoadPath;
use ecs::core::{resource::Resource, IndexMap};
use graphics::resources::{pipeline::PrimitiveState, shader::meta::ShaderMeta};

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

pub struct MaterialPipelineMeta {
    pub material: MaterialType,
    pub depth_write: DepthWrite,
    pub primitive: PrimitiveState,
    pub shader: LoadPath,
    pub meta: ShaderMeta,
}

impl MaterialPipelineMeta {
    pub fn new<M: Material, P: MaterialPipeline<M>>() -> Self {
        Self {
            material: MaterialType::of::<M>(),
            depth_write: P::depth_write(),
            primitive: P::primitive(),
            shader: P::shader().into(),
            meta: P::meta(),
        }
    }
}

pub struct MaterialRegistry {
    materials: IndexMap<MaterialType, MaterialMeta>,
    pipelines: IndexMap<MaterialPipelineKey, MaterialPipelineMeta>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self {
            materials: IndexMap::new(),
            pipelines: IndexMap::new(),
        }
    }

    pub fn register_material<M: Material>(&mut self) {
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

    pub fn register_pipeline<M: Material, P: MaterialPipeline<M>>(&mut self) {
        self.pipelines.insert(
            MaterialPipelineKey::new::<M, P>(),
            MaterialPipelineMeta::new::<M, P>(),
        );
    }

    pub fn material(&self, ty: MaterialType) -> Option<&MaterialMeta> {
        self.materials.get(&ty)
    }

    pub fn pipeline(&self, key: MaterialPipelineKey) -> Option<&MaterialPipelineMeta> {
        self.pipelines.get(&key)
    }

    pub fn materials(&self) -> impl Iterator<Item = (&MaterialType, &MaterialMeta)> {
        self.materials.iter()
    }

    pub fn pipelines(&self) -> impl Iterator<Item = (&MaterialPipelineKey, &MaterialPipelineMeta)> {
        self.pipelines.iter()
    }
}

impl Resource for MaterialRegistry {}
