pub mod bitset;
pub mod component;
pub mod entity;
pub mod internal;
pub mod resource;

pub use indexmap::*;

use std::{any::TypeId, fmt::Debug, hash::Hash};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Type(u32);
impl Type {
    pub fn of<R: 'static>() -> Self {
        let mut hasher = crc32fast::Hasher::new();
        TypeId::of::<R>().hash(&mut hasher);
        Self(hasher.finalize())
    }

    pub fn dynamic(ty: u32) -> Self {
        Self(ty)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<TypeId> for Type {
    fn from(type_id: TypeId) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        type_id.hash(&mut hasher);
        Self(hasher.finalize())
    }
}
