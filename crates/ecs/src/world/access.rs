use crate::{
    core::{
        bitset::Bitset,
        resource::{NonSend, NonSendMut, Res, ResMut, Resource, ResourceId},
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

    fn access() -> Vec<crate::system::WorldAccess> {
        vec![crate::system::WorldAccess::Resource {
            ty: ResourceId::of::<R>(),
            access: crate::system::AccessType::Read,
            send: true,
        }]
    }

    fn is_send() -> bool {
        true
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

    fn access() -> Vec<crate::system::WorldAccess> {
        vec![crate::system::WorldAccess::Resource {
            ty: ResourceId::of::<R>(),
            access: crate::system::AccessType::Write,
            send: true,
        }]
    }

    fn is_send() -> bool {
        true
    }
}

impl<R: Resource + Clone + Send> SystemArg for R {
    type Item<'a> = R;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        world.resource::<R>().clone()
    }
}

impl<R: Resource> SystemArg for NonSend<'_, R> {
    type Item<'a> = NonSend<'a, R>;

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
        world.non_send_resource::<R>()
    }

    fn done(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            world.get().access().clear(index);
        }
    }

    fn access() -> Vec<crate::system::WorldAccess> {
        vec![crate::system::WorldAccess::Resource {
            ty: ResourceId::of::<R>(),
            access: crate::system::AccessType::Read,
            send: false,
        }]
    }

    fn is_send() -> bool {
        false
    }
}

impl<R: Resource> SystemArg for NonSendMut<'_, R> {
    type Item<'a> = NonSendMut<'a, R>;

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
        world.non_send_resource_mut::<R>()
    }

    fn done(world: &super::cell::WorldCell) {
        #[cfg(debug_assertions)]
        {
            let ty = Type::of::<R>();
            let index = world.get().registry().index_of(&ty);
            world.get().access().clear(index);
        }
    }

    fn access() -> Vec<crate::system::WorldAccess> {
        vec![crate::system::WorldAccess::Resource {
            ty: ResourceId::of::<R>(),
            access: crate::system::AccessType::Write,
            send: false,
        }]
    }

    fn is_send() -> bool {
        false
    }
}

pub struct Removed<R: Resource + Send> {
    resource: Option<R>,
}

impl<R: Resource + Send> Removed<R> {
    pub fn into_inner(self) -> Option<R> {
        self.resource
    }
}

impl<R: Resource + Send> SystemArg for Removed<R> {
    type Item<'a> = Self;

    fn get<'a>(world: WorldCell<'a>) -> Self::Item<'a> {
        let resource = world.remove_resource::<R>();
        Self { resource }
    }
}

impl<R: Resource + Send> std::ops::Deref for Removed<R> {
    type Target = Option<R>;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<R: Resource + Send> std::ops::DerefMut for Removed<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource
    }
}
