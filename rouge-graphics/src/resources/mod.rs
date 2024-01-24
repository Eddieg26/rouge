use std::hash::{Hash, Hasher};

pub mod bind_group;
pub mod buffer;
pub mod material;
pub mod pipeline;
pub mod shader;
pub mod texture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn zero() -> Self {
        Self(0)
    }
}

impl From<u64> for ResourceId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<ResourceId> for u64 {
    fn from(id: ResourceId) -> Self {
        id.0
    }
}

impl std::ops::Deref for ResourceId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for ResourceId {
    fn from(name: String) -> Self {
        Self::from(name.as_str())
    }
}

impl From<&str> for ResourceId {
    fn from(name: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self(hasher.finish())
    }
}
