use crate::{
    core::{resource::Resource, Type},
    system::schedule::PhaseId,
};
use indexmap::{IndexMap, IndexSet};
use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

pub trait Event: Send + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Type);
impl EventId {
    pub fn of<E: Event>() -> Self {
        Self(Type::of::<E>())
    }
}
impl std::ops::Deref for EventId {
    type Target = Type;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<Type> for EventId {
    fn into(self) -> Type {
        self.0
    }
}

pub struct Events<E: Event> {
    events: Vec<E>,
    invoked: Arc<Mutex<IndexSet<EventId>>>,
}

impl<E: Event> Events<E> {
    pub fn new(invoked: Arc<Mutex<IndexSet<EventId>>>) -> Self {
        Self {
            events: Vec::new(),
            invoked,
        }
    }

    pub fn add(&mut self, event: E) {
        self.events.push(event);
        self.invoked.lock().unwrap().insert(EventId::of::<E>());
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = E>) {
        self.events.extend(events);
        self.invoked.lock().unwrap().insert(EventId::of::<E>());
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.events.iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.drain(..)
    }

    pub fn take(&mut self) -> Vec<E> {
        std::mem::take(&mut self.events)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl<E: Event> Resource for Events<E> {}

impl<'a, E: Event> IntoIterator for &'a Events<E> {
    type Item = &'a E;
    type IntoIter = std::slice::Iter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.iter()
    }
}

pub struct InvokedEvents {
    invoked: Arc<Mutex<IndexSet<EventId>>>,
    deferred: Arc<Mutex<IndexMap<PhaseId, IndexSet<EventId>>>>,
}

impl InvokedEvents {
    pub fn new() -> Self {
        Self {
            invoked: Arc::default(),
            deferred: Arc::default(),
        }
    }

    pub fn invoke<E: Event>(&self) {
        self.invoked.lock().unwrap().insert(EventId::of::<E>());
    }

    pub fn defer<E: Event>(&self, phase: PhaseId) {
        self.deferred
            .lock()
            .unwrap()
            .entry(phase)
            .or_default()
            .insert(EventId::of::<E>());
    }

    pub fn invoked(&self) -> &Arc<Mutex<IndexSet<EventId>>> {
        &self.invoked
    }

    pub fn deferred(&self, phase: PhaseId) -> Option<IndexSet<EventId>> {
        let mut invoked = self.deferred.lock().unwrap();
        invoked.shift_remove(&phase)
    }
}
