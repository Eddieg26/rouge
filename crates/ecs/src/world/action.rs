use super::{cell::WorldCell, World};
use crate::system::SystemArg;
use std::sync::{Arc, Mutex};

pub trait WorldAction: Send + Sync + 'static {
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
    pub fn add(&self, action: impl WorldAction) {
        self.actions
            .lock()
            .unwrap()
            .push(WorldActionFn::from(action));
    }

    pub fn len(&self) -> usize {
        self.actions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.lock().unwrap().is_empty()
    }

    pub fn drain(&self) -> Vec<WorldActionFn> {
        self.actions.lock().unwrap().drain(..).collect()
    }
}

impl SystemArg for WorldActions {
    type Item<'a> = &'a WorldActions;

    fn get<'a>(world: &'a WorldCell) -> Self::Item<'a> {
        &world.get().actions
    }
}
