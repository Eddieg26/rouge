use ecs::event::Event;
use std::sync::Arc;

pub mod asset;
pub mod pipeline;
pub mod resource;

pub use asset::*;
pub use pipeline::*;
pub use resource::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAssetWorld {
    Main,
    Render,
}

#[derive(Debug, Clone)]
pub enum ExtractError {
    MissingAsset,
    MissingDependency,
    DependencyFailed,
    Error(Arc<dyn std::error::Error + Send + Sync + 'static>),
}

impl ExtractError {
    pub fn from_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Error(Arc::new(error))
    }
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAsset => write!(f, "Missing asset"),
            Self::MissingDependency => write!(f, "Missing dependency"),
            Self::DependencyFailed => write!(f, "Dependency failed"),
            Self::Error(error) => write!(f, "{}", error),
        }
    }
}

impl std::error::Error for ExtractError {}

impl Event for ExtractError {}

pub trait RenderPipelineExtractor {}

pub trait ComputePipelineExtractor {}

pub trait RenderViewExtractor {}

pub trait DrawExtractor {}
