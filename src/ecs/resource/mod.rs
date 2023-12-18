use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub mod id;
pub mod manager;

pub use id::*;

pub trait Resource: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct Res<T: Resource> {
    resource: Rc<RefCell<Box<dyn Resource>>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Resource> Res<T> {
    pub fn new(resource: &Rc<RefCell<Box<dyn Resource>>>) -> Res<T> {
        Res {
            resource: resource.clone(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get<'a>(&'a self) -> Ref<'a, T> {
        Ref::map(self.resource.borrow(), |r| {
            r.as_any().downcast_ref::<T>().unwrap()
        })
    }

    pub fn get_mut<'a>(&'a mut self) -> RefMut<'a, T> {
        RefMut::map(self.resource.borrow_mut(), |r| {
            r.as_any_mut().downcast_mut::<T>().unwrap()
        })
    }
}
