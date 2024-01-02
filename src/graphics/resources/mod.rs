use crate::ecs::resource::ResourceId;
use std::collections::HashMap;

pub mod bind_group;
pub mod buffer;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

pub type BufferId = ResourceId;
pub type MeshId = ResourceId;
pub type TextureId = ResourceId;
pub type SamplerId = ResourceId;
pub type ShaderId = ResourceId;
pub type ShaderGraphId = ResourceId;
pub type Resources<T> = HashMap<ResourceId, T>;
