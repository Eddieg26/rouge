use crate::{
    core::{
        registry::{Record, Registry, Type},
        resource::Resource,
    },
    system::schedule::PhaseId,
    world::World,
};
use indexmap::{IndexMap, IndexSet};
use std::{
    alloc::Layout,
    any::{type_name, TypeId},
    hash::Hash,
    sync::{Arc, Mutex, MutexGuard},
};

pub trait Event: Send + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventMeta {
    name: &'static str,
    layout: Layout,
    type_id: TypeId,
    clear: fn(&World),
}

impl EventMeta {
    pub fn new<E: Event>() -> Self {
        let name = type_name::<E>();
        let layout = Layout::new::<E>();
        let type_id = TypeId::of::<E>();

        Self {
            name,
            layout,
            type_id,
            clear: |world| world.resource_mut::<Events<E>>().clear(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn type_id(&self) -> &TypeId {
        &self.type_id
    }

    pub fn clear(&self, world: &World) {
        (self.clear)(world)
    }
}

impl Record for EventMeta {
    type Type = EventId;
}

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

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.events.iter()
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

pub struct EventRegistry {
    events: Registry<EventMeta>,
    invoked: Arc<Mutex<IndexSet<EventId>>>,
    deferred: Arc<Mutex<IndexMap<PhaseId, IndexSet<EventId>>>>,
}

impl EventRegistry {
    #[inline]
    pub fn new() -> Self {
        Self {
            events: Registry::new(),
            invoked: Arc::default(),
            deferred: Arc::default(),
        }
    }

    #[inline]
    pub fn register<E: Event>(&mut self) -> Events<E> {
        self.events
            .register(EventId::of::<E>(), EventMeta::new::<E>());
        Events::new(self.invoked.clone())
    }

    #[inline]
    pub fn get(&self, ty: &EventId) -> Option<&EventMeta> {
        self.events.get(ty)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &EventMeta> {
        self.events.iter()
    }

    #[inline]
    pub fn invoke<E: Event>(&self) {
        self.invoked.lock().unwrap().insert(EventId::of::<E>());
    }

    #[inline]
    pub fn defer<E: Event>(&self, phase: PhaseId) {
        self.deferred
            .lock()
            .unwrap()
            .entry(phase)
            .or_default()
            .insert(EventId::of::<E>());
    }

    #[inline]
    pub fn invoked(&self) -> MutexGuard<IndexSet<EventId>> {
        self.invoked.lock().unwrap()
    }

    #[inline]
    pub fn deferred(&self, phase: PhaseId) -> Option<IndexSet<EventId>> {
        let mut invoked = self.deferred.lock().unwrap();
        invoked.shift_remove(&phase)
    }
}
