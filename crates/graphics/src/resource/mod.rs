use asset::AssetId;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

pub mod binding;
pub mod buffer;
pub mod mesh;
pub mod pipeline;
pub mod shader;
pub mod texture;

pub use binding::*;
pub use buffer::*;
pub use mesh::*;
pub use pipeline::*;
pub use shader::*;
pub use texture::*;

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

pub enum Handle<T> {
    Ref(Id<T>),
    Owned(T),
}

impl<T> Handle<T> {
    pub fn id(&self) -> Option<&Id<T>> {
        match self {
            Handle::Ref(id) => Some(id),
            _ => None,
        }
    }

    pub fn owned(&self) -> Option<&T> {
        match self {
            Handle::Owned(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Handle::Ref(id) => write!(f, "{:?}", id),
            Handle::Owned(value) => write!(f, "{:?}", value),
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Handle::Ref(id) => write!(f, "{:?}", id),
            Handle::Owned(value) => write!(f, "{}", value),
        }
    }
}

impl<T> From<Id<T>> for Handle<T> {
    fn from(value: Id<T>) -> Self {
        Handle::Ref(value)
    }
}

impl<T> From<T> for Handle<T> {
    fn from(value: T) -> Self {
        Handle::Owned(value)
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

pub type Label = Option<Cow<'static, str>>;
