use super::World;
use crate::{
    archetype::Archetypes,
    core::resource::{Res, ResMut, Resource},
};
use std::marker::PhantomData;

/// Provides an unsafe way to access world components and resources.
/// Users must ensure that the Rust's borrowing rules are not violated.
#[derive(Clone, Copy)]
pub struct WorldCell<'a>(*mut World, PhantomData<&'a mut World>);

impl<'a> WorldCell<'a> {
    #[inline]
    pub fn get(&self) -> &'a World {
        unsafe { &*self.0 }
    }

    #[inline]
    pub fn get_mut(&self) -> &'a mut World {
        unsafe { &mut *self.0 }
    }

    #[inline]
    pub fn resource<R: Resource + Send>(&self) -> Res<R> {
        Res::new(self.get().resource::<R>())
    }

    #[inline]
    pub fn resource_mut<R: Resource + Send>(&self) -> ResMut<R> {
        ResMut::new(self.get_mut().resource_mut::<R>())
    }

    #[inline]
    pub fn try_resource<R: Resource + Send>(&self) -> Option<Res<R>> {
        Some(Res::new(self.get().try_resource::<R>()?))
    }

    #[inline]
    pub fn try_resource_mut<R: Resource + Send>(&self) -> Option<ResMut<R>> {
        Some(ResMut::new(self.get_mut().try_resource_mut::<R>()?))
    }

    #[inline]
    pub fn non_send_resource<R: Resource>(&self) -> Res<R> {
        Res::new(self.get().non_send_resource::<R>())
    }

    #[inline]
    pub fn non_send_resource_mut<R: Resource>(&self) -> ResMut<R> {
        ResMut::new(self.get_mut().non_send_resource_mut::<R>())
    }

    #[inline]
    pub fn try_non_send_resource<R: Resource>(&self) -> Option<Res<R>> {
        Some(Res::new(self.get().try_non_send_resource::<R>()?))
    }

    #[inline]
    pub fn try_non_send_resource_mut<R: Resource>(&self) -> Option<ResMut<R>> {
        Some(ResMut::new(
            self.get_mut().try_non_send_resource_mut::<R>()?,
        ))
    }
}

impl<'a> From<&mut World> for WorldCell<'a> {
    fn from(world: &mut World) -> Self {
        WorldCell(world as *const _ as *mut _, PhantomData)
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

pub struct ArchetypeCell<'a>(*mut Archetypes, PhantomData<&'a mut Archetypes>);

impl<'a> ArchetypeCell<'a> {}
