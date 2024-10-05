use super::World;
use std::marker::PhantomData;

/// Provides an unsafe way to access world components and resources.
/// Users must ensure that the Rust's borrowing rules are not violated.
pub struct WorldCell<'a>(*mut World, PhantomData<&'a mut World>);

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

impl<'a> WorldCell<'a> {
    #[inline]
    pub fn get(&self) -> &'a World {
        unsafe { &*self.0 }
    }

    #[inline]
    pub fn get_mut(&self) -> &'a mut World {
        unsafe { &mut *self.0 }
    }
}

unsafe impl<'a> Send for WorldCell<'a> {}
unsafe impl<'a> Sync for WorldCell<'a> {}
