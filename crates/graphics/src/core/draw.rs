use bytemuck::Pod;
use ecs::core::{entity::Entity, IndexMap};
use std::hash::Hash;

pub trait Draw: Sized + Send + Sync + 'static {
    const SORT: bool;
    const CULL: bool;
    
    type Depth: Ord + 'static;
    type Bounds: 'static;

    fn entity(&self) -> Entity;
    fn depth(&self) -> Self::Depth;
}

pub trait BatchDraw: Draw {
    type Key: Copy + Eq + Hash + Send + Sync + 'static;
    type BatchData: 'static;
    type InstanceData: Pod + 'static;

    fn key(&self) -> Self::Key;
    fn data(&self) -> Self::BatchData;
    fn instance_data(&self) -> Option<Self::InstanceData>;
    fn can_batch(&self) -> bool;
}

pub trait RenderView: 'static {}

pub struct DrawCalls<D: Draw> {
    calls: Vec<D>,
}

pub struct BatchDrawCalls<D: BatchDraw> {
    batches: IndexMap<usize, D::Key>,
    batched: Vec<D>,
    unbatched: Vec<D>,
}
