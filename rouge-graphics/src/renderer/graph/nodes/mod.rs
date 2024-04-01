use super::context::RenderContext;
use downcast_rs::{impl_downcast, Downcast};
use rouge_ecs::meta::AccessMeta;
use std::hash::{Hash, Hasher};

pub mod compute;
pub mod render;
pub mod present;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RenderPhase {
    Process,
    PostProcess,
    Finish,
    Present
}

impl RenderPhase {
    pub fn iter() -> impl Iterator<Item = RenderPhase> {
        RenderPhase::Process.into_iter()
    }
}

impl From<usize> for RenderPhase {
    fn from(value: usize) -> Self {
        match value {
            0 => RenderPhase::Process,
            1 => RenderPhase::PostProcess,
            2 => RenderPhase::Finish,
            3 => RenderPhase::Present,
            _ => panic!("Invalid RenderPhase value"),
        }
    }
}

impl Iterator for RenderPhase {
    type Item = RenderPhase;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RenderPhase::Process => {
                *self = RenderPhase::PostProcess;
                Some(RenderPhase::Process)
            }
            RenderPhase::PostProcess => {
                *self = RenderPhase::Finish;
                Some(RenderPhase::PostProcess)
            }
            RenderPhase::Finish => {
                *self = RenderPhase::Present;
                Some(RenderPhase::Finish)
            },
            RenderPhase::Present => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(id: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl From<&str> for NodeId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<&String> for NodeId {
    fn from(id: &String) -> Self {
        Self::new(id)
    }
}

pub trait GraphNode: Downcast + Send + Sync + 'static {
    fn prepare(&mut self, _: RenderContext) {}
    fn execute(&self, ctx: RenderContext);
    fn phase(&self) -> RenderPhase {
        RenderPhase::Process
    }
    fn access(&self) -> Vec<AccessMeta> {
        vec![]
    }
}

impl_downcast!(GraphNode);
