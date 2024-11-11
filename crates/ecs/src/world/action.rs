use super::{cell::WorldCell, World};
use crate::{
    event::{Event, Events},
    system::SystemArg,
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
}

impl WorldActions {
    pub fn add(&self, action: impl Into<WorldActionFn>) {
        self.actions
            .lock()
            .unwrap()
            .push(WorldActionFn::from(action.into()));
    }

    pub fn len(&self) -> usize {
        self.actions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.lock().unwrap().is_empty()
    }

    pub fn extend(&self, mut actions: Vec<impl Into<WorldActionFn>>) {
        let mut list = self.actions.lock().unwrap();
        list.extend(actions.drain(..).map(|a| a.into()));
    }

    pub fn take(&self) -> Vec<WorldActionFn> {
        std::mem::take(&mut self.actions.lock().unwrap())
    }
}

impl SystemArg for &WorldActions {
    type Item<'a> = &'a WorldActions;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        &world.get().actions
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
