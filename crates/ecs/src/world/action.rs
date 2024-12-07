use hashbrown::HashMap;

use super::{cell::WorldCell, World};
use crate::{
    event::{Event, Events},
    system::{
        schedule::{Phase, PhaseId},
        SystemArg,
    },
};
use std::sync::{Arc, Mutex};

pub trait WorldAction: Send + 'static {
    fn execute(self, world: &mut World) -> Option<()>;
}

pub struct WorldActionFn(Box<dyn FnOnce(&mut World) -> Option<()> + Send>);
impl WorldActionFn {
    pub fn execute(self, world: &mut World) {
        (self.0)(world);
    }
}
impl<W: WorldAction> From<W> for WorldActionFn {
    fn from(action: W) -> Self {
        Self(Box::new(move |world| action.execute(world)))
    }
}

#[derive(Default, Clone)]
pub struct WorldActions {
    actions: Arc<Mutex<Vec<WorldActionFn>>>,
    deferred: Arc<Mutex<HashMap<PhaseId, Vec<WorldActionFn>>>>,
}

impl WorldActions {
    pub fn add(&self, action: impl Into<WorldActionFn>) {
        self.actions
            .lock()
            .unwrap()
            .push(WorldActionFn::from(action.into()));
    }

    pub fn defer<P: Phase>(&self, action: impl Into<WorldActionFn>) {
        let mut deferred = self.deferred.lock().unwrap();
        deferred
            .entry(PhaseId::of::<P>())
            .or_insert(vec![])
            .push(action.into());
    }

    pub fn len(&self) -> usize {
        self.actions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.lock().unwrap().is_empty()
    }

    pub fn extend(&self, actions: impl IntoIterator<Item = impl Into<WorldActionFn>>) {
        let mut list = self.actions.lock().unwrap();
        list.extend(actions.into_iter().map(|a| a.into()));
    }

    pub fn extend_deferred<P: Phase>(
        &mut self,
        actions: impl IntoIterator<Item = impl Into<WorldActionFn>>,
    ) {
        let mut deferred = self.deferred.lock().unwrap();
        deferred
            .entry(PhaseId::of::<P>())
            .or_insert(vec![])
            .extend(actions.into_iter().map(|a| a.into()));
    }

    pub fn take(&self, phase: Option<PhaseId>) -> Vec<WorldActionFn> {
        let mut actions = phase
            .map(|phase| {
                let mut deferred = self.deferred.lock().unwrap();
                deferred.remove(&phase).unwrap_or(vec![])
            })
            .unwrap_or_default();

        actions.extend(self.actions.lock().unwrap().drain(..));
        actions
    }
}

impl SystemArg for &WorldActions {
    type Item<'a> = &'a WorldActions;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        &world.get().actions
    }
}

impl SystemArg for WorldActions {
    type Item<'a> = Self;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.get().actions.clone()
    }
}

pub struct BatchEvents<E: Event> {
    events: Vec<E>,
}

impl<E: Event> BatchEvents<E> {
    pub fn new(events: impl IntoIterator<Item = E>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    pub fn into_inner(self) -> Vec<E> {
        self.events
    }
}

impl<E: Event> WorldAction for BatchEvents<E> {
    fn execute(self, world: &mut World) -> Option<()> {
        let events = world.resource_mut::<Events<E>>();
        Some(events.extend(self.events))
    }
}
