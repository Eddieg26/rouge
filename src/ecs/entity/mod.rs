pub mod registry;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use super::HashId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectState {
    Created,
    Enabled,
    Disabled,
    Destroyed,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u64);

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl EntityId {
    pub fn new(id: u64) -> EntityId {
        EntityId(id)
    }

    pub fn uuid() -> EntityId {
        EntityId(HashId::new())
    }

    pub fn zero() -> EntityId {
        EntityId(0)
    }
}

impl Deref for EntityId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EntityId {
    fn deref_mut(&mut self) -> &mut u64 {
        &mut self.0
    }
}
