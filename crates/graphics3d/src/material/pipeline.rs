use super::MaterialType;
use asset::{io::cache::LoadPath, AssetId};
use ecs::core::{IndexMap, Type};
use graphics::resources::{
    pipeline::{PrimitiveState, RenderPipeline},
    shader::meta::ShaderMeta,
};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait MeshPipelineConfig: 'static {
    fn depth_write() -> DepthWrite;
    fn primitive() -> PrimitiveState;
    fn shader() -> impl Into<LoadPath>;
    fn meta() -> ShaderMeta;
}

pub type MaterialPipelines = IndexMap<MaterialType, RenderPipeline>;

pub struct MeshPipeline {
    depth_write: DepthWrite,
    primitive: PrimitiveState,
    shader: LoadPath,
    meta: ShaderMeta,
    pipelines: MaterialPipelines,
}

impl MeshPipeline {
    pub fn new<P: MeshPipelineConfig>() -> Self {
        Self {
            depth_write: P::depth_write(),
            primitive: P::primitive(),
            shader: P::shader().into(),
            meta: P::meta(),
            pipelines: MaterialPipelines::new(),
        }
    }

    pub fn depth_write(&self) -> DepthWrite {
        self.depth_write
    }

    pub fn primitive(&self) -> PrimitiveState {
        self.primitive
    }

    pub fn shader(&self) -> &LoadPath {
        &self.shader
    }

    pub fn meta(&self) -> &ShaderMeta {
        &self.meta
    }

    pub fn has_pipeline(&self, ty: MaterialType) -> bool {
        self.pipelines.contains_key(&ty)
    }

    pub fn pipeline(&self, ty: MaterialType) -> Option<&RenderPipeline> {
        self.pipelines.get(&ty)
    }

    pub fn pipelines(&self) -> impl Iterator<Item = (&MaterialType, &RenderPipeline)> {
        self.pipelines.iter()
    }

    pub fn add_pipeline(&mut self, ty: MaterialType, pipeline: RenderPipeline) {
        self.pipelines.insert(ty, pipeline);
    }

    fn remove_pipeline(&mut self, ty: MaterialType) {
        self.pipelines.shift_remove(&ty);
    }
}

pub struct MeshPipelines {
    pipelines: IndexMap<Type, MeshPipeline>,
    dependencies: IndexMap<MaterialType, HashSet<AssetId>>,
}

impl MeshPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: IndexMap::new(),
            dependencies: IndexMap::new(),
        }
    }

    pub fn register<P: MeshPipelineConfig>(&mut self) {
        self.pipelines
            .insert(Type::of::<P>(), MeshPipeline::new::<P>());
    }

    pub fn get<P: MeshPipelineConfig>(&self) -> Option<&MeshPipeline> {
        self.pipelines.get(&Type::of::<P>())
    }

    pub fn get_mut<P: MeshPipelineConfig>(&mut self) -> Option<&mut MeshPipeline> {
        self.pipelines.get_mut(&Type::of::<P>())
    }

    pub fn get_dyn(&self, ty: Type) -> Option<&MeshPipeline> {
        self.pipelines.get(&ty)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Type, &MeshPipeline)> {
        self.pipelines.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Type, &mut MeshPipeline)> {
        self.pipelines.iter_mut()
    }

    pub fn add_dependency(&mut self, ty: MaterialType, id: AssetId) {
        self.dependencies.entry(ty).or_default().insert(id);
    }

    pub fn remove_dependency(&mut self, ty: MaterialType, id: AssetId) {
        let remove = match self.dependencies.get_mut(&ty) {
            Some(dependencies) => {
                dependencies.remove(&id);
                dependencies.is_empty()
            }
            None => false,
        };

        if remove {
            self.dependencies.shift_remove(&ty);
            for pipeline in self.pipelines.values_mut() {
                pipeline.remove_pipeline(ty);
            }
        }
    }

    pub fn clear(&mut self) {
        for pipeline in self.pipelines.values_mut() {
            pipeline.pipelines.clear();
        }
        self.dependencies.clear();
    }
}
