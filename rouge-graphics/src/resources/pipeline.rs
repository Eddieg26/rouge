use rouge_core::ResourceId;
use rouge_ecs::storage::sparse::SparseMap;
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
};
use wgpu::PrimitiveState;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DepthWrite {
    Auto,
    On,
    Off,
}

pub trait Pipeline: 'static {
    fn vertex() -> ResourceId;
    fn depth_write() -> DepthWrite;
    fn primitive() -> PrimitiveState;
}

pub struct PipelineConfig {
    vertex: ResourceId,
    depth_write: DepthWrite,
    primitive: wgpu::PrimitiveState,
}

impl PipelineConfig {
    pub fn new<P: Pipeline>() -> Self {
        Self {
            vertex: P::vertex(),
            depth_write: P::depth_write(),
            primitive: P::primitive(),
        }
    }

    pub fn vertex(&self) -> ResourceId {
        self.vertex
    }

    pub fn depth_write(&self) -> DepthWrite {
        self.depth_write
    }

    pub fn primitive(&self) -> wgpu::PrimitiveState {
        self.primitive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(u64);

impl PipelineId {
    pub fn new<P: Pipeline>(fragment: ResourceId) -> PipelineId {
        let type_id = std::any::TypeId::of::<P>();
        Self::raw(type_id, fragment)
    }

    pub fn raw(type_id: TypeId, fragment: ResourceId) -> PipelineId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        type_id.hash(&mut hasher);
        fragment.hash(&mut hasher);

        Self(hasher.finish())
    }
}

pub struct Pipelines {
    configs: SparseMap<TypeId, PipelineConfig>,
    pipelines: SparseMap<PipelineId, wgpu::RenderPipeline>,
}

impl Pipelines {
    pub fn new() -> Self {
        Self {
            configs: SparseMap::new(),
            pipelines: SparseMap::new(),
        }
    }

    pub fn register<P: Pipeline>(&mut self) {
        let config = PipelineConfig::new::<P>();
        let type_id = TypeId::of::<P>();

        self.configs.insert(type_id, config);
    }

    pub fn get<P: Pipeline>(&self, fragment: ResourceId) -> Option<&wgpu::RenderPipeline> {
        let pipeline_id = PipelineId::new::<P>(fragment);

        self.pipelines.get(&pipeline_id)
    }

    pub fn remove(&mut self, fragment: ResourceId) -> Option<wgpu::RenderPipeline> {
        for (id, _) in self.configs.iter() {
            let pipeline_id = PipelineId::raw(*id, fragment);

            if let Some(pipeline) = self.pipelines.remove(&pipeline_id) {
                return Some(pipeline);
            }
        }

        None
    }

    pub fn insert(&mut self, id: PipelineId, pipeline: wgpu::RenderPipeline) {
        self.pipelines.insert(id, pipeline);
    }
}
