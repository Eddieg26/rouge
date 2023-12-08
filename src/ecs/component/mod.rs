pub mod manager;
pub mod registry;

use std::{
    any::{Any, TypeId},
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};
pub trait Component: Any + Sized + 'static {}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash, PartialOrd, Ord)]
pub struct ComponentType(u64);

impl ComponentType {
    pub fn new(_type: u64) -> ComponentType {
        ComponentType(_type)
    }

    pub fn zero() -> ComponentType {
        ComponentType(0)
    }
}

impl From<u64> for ComponentType {
    fn from(value: u64) -> Self {
        ComponentType(value)
    }
}

impl From<TypeId> for ComponentType {
    fn from(value: TypeId) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);

        ComponentType(hasher.finish())
    }
}

impl Deref for ComponentType {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ComponentType {
    fn deref_mut(&mut self) -> &mut u64 {
        &mut self.0
    }
}
