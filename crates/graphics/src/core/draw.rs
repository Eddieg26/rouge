use bytemuck::Pod;
use ecs::core::{entity::Entity, IndexMap};
use std::hash::Hash;

pub trait Draw: Sized + Send + Sync + 'static {
    type Depth: Ord + 'static;

    fn entity(&self) -> Entity;
    fn depth(&self) -> Self::Depth;
    fn sort() -> bool;
}

pub trait BatchDraw: Draw {
    type Key: Copy + Eq + Hash + 'static;
    type BatchData: 'static;
    type InstanceData: Pod + 'static;

    fn key(&self) -> Self::Key;
    fn data(&self) -> Self::BatchData;
    fn instance_data(&self) -> Option<Self::InstanceData>;
    fn can_batch(&self) -> bool;
}

pub struct DrawCalls<D: Draw> {
    calls: Vec<D>,
}

pub struct Batch<D: BatchDraw> {
    data: D::BatchData,
    instances: Vec<D::InstanceData>,
}

pub struct BatchDrawCalls<D: BatchDraw> {
    batches: IndexMap<D::Key, Batch<D>>,
    calls: Vec<D>,
}
