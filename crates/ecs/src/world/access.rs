use crate::{
    core::{
        bitset::Bitset,
        resource::{Res, ResMut, Resource},
        Type,
    },
    system::SystemArg,
};
use std::sync::{Arc, Mutex};

use super::cell::WorldCell;

#[derive(Debug, Clone)]
pub struct WorldAccessTracker {
    access: Arc<Mutex<Bitset>>,
}

impl WorldAccessTracker {
    pub fn new() -> Self {
        Self {
            access: Arc::new(Mutex::new(Bitset::new())),
        }
    }

    pub fn read(&self, index: usize) -> bool {
        let mut access = self.access.lock().unwrap();
        let index = index * 2;
        if access.get(index + 1) {
            return false;
        } else {
            access.set(index);
            return true;
        }
    }

    pub fn write(&self, index: usize) -> bool {
        let mut access = self.access.lock().unwrap();
        let index = index * 2;
        if access.get(index) || access.get(index + 1) {
            return false;
        } else {
            access.set(index + 1);
            return true;
        }
    }

    pub fn clear(&self, index: usize) {
        let mut access = self.access.lock().unwrap();
        let index = index * 2;
        access.clear(index);
        access.clear(index + 1);
    }

    pub fn clear_all(&self) {
        let mut access = self.access.lock().unwrap();
        access.clear_all();
    }
}

impl<R: Resource + Send> SystemArg for Res<'_, R> {
    type Item<'a> = Res<'a, R>;

    fn init(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            if !world.get().access().read(index) {
                let meta = world.get().registry().get(&ty);
                panic!("Resource {} is already borrowed mutably", meta.name());
            }
        }
    }

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.resource::<R>()
    }

    fn done(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            world.get().access().clear(index);
        }
    }
}

impl<R: Resource + Send> SystemArg for ResMut<'_, R> {
    type Item<'a> = ResMut<'a, R>;

    fn init(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            if !world.get().access().write(index) {
                let meta = world.get().registry().get(&ty);
                panic!("Resource {} is already borrowed", meta.name());
            }
        }
    }

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.resource_mut::<R>()
    }

    fn done(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            world.get().access().clear(index);
        }
    }
}
