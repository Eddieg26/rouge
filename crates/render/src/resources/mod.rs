use asset::AssetId;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

pub mod binding;
pub mod buffer;
pub mod extract;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

pub use binding::*;
pub use buffer::*;
pub use extract::*;
pub use mesh::*;
pub use pipeline::*;
pub use shader::*;
pub use texture::*;

pub type Label = Option<Cow<'static, str>>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Id<T> {
    id: u128,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Id<T> {
    pub const fn new(value: u128) -> Self {
        Self {
            id: value,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn generate() -> Self {
        let id = uuid::Uuid::new_v4().as_u128();
        Self::new(id)
    }

    pub fn to<S>(&self) -> Id<S> {
        Id::new(self.id)
    }
}

impl<T> From<String> for Id<T> {
    fn from(value: String) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        value.hash(&mut hasher);
        let id = hasher.finish();
        Self::new(id.into())
    }
}

impl<T> From<&str> for Id<T> {
    fn from(value: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        value.hash(&mut hasher);
        let id = hasher.finish();
        Self::new(id.into())
    }
}

impl<T> From<AssetId> for Id<T> {
    fn from(value: AssetId) -> Self {
        Self::new(value.as_u128())
    }
}

impl<T> From<&AssetId> for Id<T> {
    fn from(value: &AssetId) -> Self {
        Self::new(value.as_u128())
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.id)
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.id)
    }
}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Id<T> {
    pub fn id(&self) -> u128 {
        self.id
    }
}

pub struct AtomicId<T> {
    id: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T> AtomicId<T> {
    pub fn new() -> Self {
        use core::sync::atomic::{AtomicU32, Ordering};

        static CURRENT_ID: AtomicU32 = AtomicU32::new(0);

        let id = CURRENT_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl<T> From<AtomicId<T>> for u32 {
    fn from(value: AtomicId<T>) -> Self {
        value.id
    }
}

impl<T> From<u32> for AtomicId<T> {
    fn from(value: u32) -> Self {
        Self {
            id: value,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Clone for AtomicId<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for AtomicId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.id)
    }
}

impl<T> Copy for AtomicId<T> {}

impl<T> PartialEq for AtomicId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for AtomicId<T> {}

impl<T> Hash for AtomicId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> PartialOrd for AtomicId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<T> Ord for AtomicId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

pub trait IntoOptionalId<T> {
    fn into_optional_id(self) -> Option<Id<T>>;
}

impl<T, S> IntoOptionalId<T> for Id<S> {
    fn into_optional_id(self) -> Option<Id<T>> {
        Some(self.to())
    }
}

impl<T, S> IntoOptionalId<T> for Option<Id<S>> {
    fn into_optional_id(self) -> Option<Id<T>> {
        self.map(|id| id.to())
    }
}

impl<T, S> IntoOptionalId<T> for Option<&Id<S>> {
    fn into_optional_id(self) -> Option<Id<T>> {
        self.map(|id| id.to())
    }
}

impl<T, S> IntoOptionalId<T> for &S
where
    for<'a> &'a S: Into<Id<T>>,
{
    fn into_optional_id(self) -> Option<Id<T>> {
        Some(self.into())
    }
}
