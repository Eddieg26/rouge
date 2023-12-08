use super::{registry::ComponentRegistry, Component, ComponentType};
use crate::ecs::{registry::Registry, resource::Resource, EntityId};
use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};

type Components = HashMap<ComponentType, Rc<RefCell<Box<dyn Registry>>>>;

#[derive(Clone)]
pub struct ComponentManager {
    components: Components,
}

impl ComponentManager {
    pub fn new(components: Components) -> ComponentManager {
        ComponentManager { components }
    }

    pub fn extend(&mut self, manager: ComponentManager) {
        self.components.extend(manager.components)
    }

    pub fn register<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>().into();
        let storage = Box::new(ComponentRegistry::<T>::new());

        self.components
            .insert(type_id, Rc::new(RefCell::new(storage)));
    }

    pub fn registry<T: Component>(&self) -> Ref<'_, ComponentRegistry<T>> {
        let id: ComponentType = TypeId::of::<T>().into();
        let components = self
            .components
            .get(&id)
            .expect("Component Registry not found.")
            .borrow();

        Ref::map(components, |x| {
            x.as_any().downcast_ref::<ComponentRegistry<T>>().unwrap()
        })
    }

    pub fn registry_mut<T: Component>(&self) -> RefMut<'_, ComponentRegistry<T>> {
        let id: ComponentType = TypeId::of::<T>().into();
        let components = self
            .components
            .get(&id)
            .expect("Component Registry not found.")
            .borrow_mut();

        RefMut::map(components, |x| {
            x.as_any_mut()
                .downcast_mut::<ComponentRegistry<T>>()
                .unwrap()
        })
    }

    pub fn registry_ref(&self, type_id: &ComponentType) -> &Rc<RefCell<Box<dyn Registry>>> {
        self.components
            .get(type_id)
            .expect("Component Registry not found.")
    }

    pub fn update(&self) {
        for (_, registry) in self.components.iter() {
            registry.borrow_mut().update();
        }
    }

    pub fn enable(&self, id: &EntityId) {
        for (_, registry) in self.components.iter() {
            registry.borrow_mut().enable(id);
        }
    }

    pub fn disable(&self, id: &EntityId) {
        for (_, registry) in self.components.iter() {
            registry.borrow_mut().disable(id);
        }
    }

    pub fn destroy(&self, id: &EntityId) {
        for (_, registry) in self.components.iter() {
            registry.borrow_mut().remove(id);
        }
    }
}

impl Resource for ComponentManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
