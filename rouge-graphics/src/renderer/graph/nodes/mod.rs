use super::context::RenderContext;
use crate::core::ResourceId;
use downcast_rs::{impl_downcast, Downcast};
use std::hash::{Hash, Hasher};

pub mod compute;
pub mod render;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(id: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub trait GraphNode: Downcast + Send + Sync + 'static {
    fn prepare(&mut self, _: RenderContext) {}
    fn execute(&self, ctx: RenderContext);
    fn reads(&self) -> Vec<ResourceId>;
    fn writes(&self) -> Vec<ResourceId>;
}

impl_downcast!(GraphNode);
