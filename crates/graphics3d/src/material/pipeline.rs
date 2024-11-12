use crate::material::Material;
use asset::io::cache::LoadPath;
use ecs::core::IndexMap;
use graphics::resources::{
    pipeline::{PrimitiveState, RenderPipeline},
    shader::meta::ShaderMeta,
};
use std::{any::TypeId, hash::Hash};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DepthWrite {
    On,
    Off,
}

pub trait MaterialPipeline<M: Material>: 'static {
    fn depth_write() -> DepthWrite;
    fn primitive() -> PrimitiveState;
    fn shader() -> impl Into<LoadPath>;
    fn meta() -> ShaderMeta;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialPipelineKey(u32);

impl MaterialPipelineKey {
    pub fn new<M: Material, P: MaterialPipeline<M>>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        TypeId::of::<M>().hash(&mut hasher);
        TypeId::of::<P>().hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct MaterialPipelineRef {
    pipeline: RenderPipeline,
    ref_count: usize,
}

pub struct MaterialPipelines {
    pipelines: IndexMap<MaterialPipelineKey, MaterialPipelineRef>,
}

impl MaterialPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: IndexMap::new(),
        }
    }

    pub fn get(&self, key: MaterialPipelineKey) -> Option<&MaterialPipelineRef> {
        self.pipelines.get(&key)
    }

    pub fn has(&self, key: MaterialPipelineKey) -> bool {
        self.pipelines.contains_key(&key)
    }

    pub fn add(&mut self, key: MaterialPipelineKey, pipeline: RenderPipeline) {
        self.pipelines.insert(
            key,
            MaterialPipelineRef {
                pipeline,
                ref_count: 1,
            },
        );
    }

    pub fn reference(&mut self, key: &MaterialPipelineKey) {
        if let Some(pipeline) = self.pipelines.get_mut(key) {
            pipeline.ref_count += 1;
        }
    }

    pub fn remove(&mut self, key: &MaterialPipelineKey) -> Option<RenderPipeline> {
        let remove = match self.pipelines.get_mut(key) {
            Some(pipeline) => {
                pipeline.ref_count -= 1;
                pipeline.ref_count == 0
            }
            None => false,
        };

        match remove {
            true => self
                .pipelines
                .shift_remove(key)
                .map(|pipeline| pipeline.pipeline),
            false => None,
        }
    }
}
