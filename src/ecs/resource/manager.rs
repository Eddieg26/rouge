use super::{Resource, ResourceType};
use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};

pub struct ResourceManager {
    resources: HashMap<ResourceType, Rc<RefCell<Box<dyn Resource>>>>,
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            resources: HashMap::new(),
        }
    }

    pub fn extend(&mut self, manager: ResourceManager) {
        self.resources.extend(manager.resources)
    }

    pub fn register<T: Resource>(&mut self, resource: T) {
        let id = TypeId::of::<T>().into();

        self.resources
            .insert(id, Rc::new(RefCell::new(Box::new(resource))));
    }

    pub fn resource<T: Resource>(&self) -> Ref<'_, T> {
        let id = TypeId::of::<T>().into();
        let resource = self
            .resources
            .get(&id)
            .expect("Resource not found.")
            .borrow();

        Ref::map(resource, |x| x.as_any().downcast_ref::<T>().unwrap())
    }

    pub fn resource_mut<T: Resource>(&self) -> RefMut<'_, T> {
        let id = TypeId::of::<T>().into();
        let resource = self
            .resources
            .get(&id)
            .expect("Resource not found.")
            .borrow_mut();

        RefMut::map(resource, |x| x.as_any_mut().downcast_mut::<T>().unwrap())
    }

    pub fn resource_ref(&self, type_id: &ResourceType) -> &Rc<RefCell<Box<dyn Resource>>> {
        self.resources.get(type_id).expect("Resource not found")
    }
}
