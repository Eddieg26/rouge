use super::World;
use crate::core::resource::{NonSend, NonSendMut, Res, ResMut, Resource};
use std::marker::PhantomData;

/// Provides an unsafe way to access world components and resources.
/// Users must ensure that the Rust's borrowing rules are not violated.
#[derive(Clone, Copy)]
pub struct WorldCell<'a>(*mut World, PhantomData<&'a mut World>);

impl<'a> WorldCell<'a> {
    pub fn get(&self) -> &'a World {
        unsafe { &*self.0 }
    }

    pub fn get_mut(&self) -> &'a mut World {
        unsafe { &mut *self.0 }
    }

    pub fn resource<R: Resource + Send>(&self) -> Res<'a, R> {
        Res::new(self.get().resource::<R>())
    }

    pub fn resource_mut<R: Resource + Send>(&self) -> ResMut<'a, R> {
        ResMut::new(self.get_mut().resource_mut::<R>())
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<Res<'a, R>> {
        Some(Res::new(self.get().try_resource::<R>()?))
    }

    pub fn try_resource_mut<R: Resource + Send>(&self) -> Option<ResMut<'a, R>> {
        Some(ResMut::new(self.get_mut().try_resource_mut::<R>()?))
    }

    pub fn non_send_resource<R: Resource>(&self) -> NonSend<'a, R> {
        NonSend::new(self.get().non_send_resource::<R>())
    }

    pub fn non_send_resource_mut<R: Resource>(&self) -> NonSendMut<'a, R> {
        NonSendMut::new(self.get_mut().non_send_resource_mut::<R>())
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<NonSend<'a, R>> {
        Some(NonSend::new(self.get().try_non_send_resource::<R>()?))
    }

    pub fn try_non_send_resource_mut<R: Resource>(&self) -> Option<NonSendMut<'a, R>> {
        Some(NonSendMut::new(
            self.get_mut().try_non_send_resource_mut::<R>()?,
        ))
    }

    pub fn remove_resource<R: Resource + Send>(&self) -> Option<R> {
        self.get_mut().remove_resource::<R>()
    }

    pub fn has_resource<R: Resource + Send>(&self) -> bool {
        self.get().has_resource::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&self) -> Option<R> {
        self.get_mut().remove_non_send_resource::<R>()
    }

    pub fn has_non_send_resource<R: Resource>(&self) -> bool {
        self.get().has_non_send_resource::<R>()
    }
}

impl<'a> From<&mut World> for WorldCell<'a> {
    fn from(world: &mut World) -> Self {
        WorldCell(world as *const _ as *mut _, PhantomData)
    }
}

impl<'a> From<&&mut World> for WorldCell<'a> {
    fn from(world: &&mut World) -> Self {
        WorldCell(*world as *const _ as *mut _, PhantomData)
    }
}

impl<'a> From<&&World> for WorldCell<'a> {
    fn from(world: &&World) -> Self {
        WorldCell(*world as *const _ as *mut _, PhantomData)
    }
}

impl<'a> From<&World> for WorldCell<'a> {
    fn from(world: &World) -> Self {
        WorldCell(world as *const _ as *mut _, PhantomData)
    }
}

impl<'a> WorldCell<'a> {}

unsafe impl<'a> Send for WorldCell<'a> {}
unsafe impl<'a> Sync for WorldCell<'a> {}
