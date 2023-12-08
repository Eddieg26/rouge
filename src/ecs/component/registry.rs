use super::Component;
use crate::ecs::{entity::EntityId, registry::Registry};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

pub enum QueryTarget {
    All,
    Created,
    Enabled,
    Disabled,
    Destroyed,
}

pub struct ComponentRegistry<T: Component> {
    components: HashMap<EntityId, T>,
    created: HashSet<EntityId>,
    enabled: HashSet<EntityId>,
    disabled: HashSet<EntityId>,
    destroyed: HashSet<EntityId>,
}

impl<T: Component> ComponentRegistry<T> {
    pub fn new() -> ComponentRegistry<T> {
        ComponentRegistry {
            components: HashMap::new(),
            created: HashSet::new(),
            destroyed: HashSet::new(),
            disabled: HashSet::new(),
            enabled: HashSet::new(),
        }
    }

    pub fn get(&self, id: &EntityId) -> Option<&T> {
        self.components.get(id)
    }

    pub fn get_mut(&mut self, id: &EntityId) -> Option<&mut T> {
        if self.destroyed.contains(id) || self.disabled.contains(id) {
            None
        } else {
            self.components.get_mut(id)
        }
    }

    pub fn enabled(&self) -> impl Iterator<Item = &EntityId> {
        self.enabled.iter()
    }

    pub fn disabled(&self) -> impl Iterator<Item = &EntityId> {
        self.disabled.iter()
    }

    pub fn destroyed(&self) -> impl Iterator<Item = &EntityId> {
        self.destroyed.iter()
    }

    pub fn all<'a>(&'a self) -> impl Iterator<Item = (&EntityId, &T)> {
        self.components.iter()
    }

    pub fn all_mut<'a>(&'a mut self) -> impl Iterator<Item = (&EntityId, &mut T)> {
        self.components.iter_mut()
    }

    pub fn extend<'a>(&'a mut self, iter: impl Iterator<Item = (EntityId, T)>) {
        self.components.extend(iter)
    }

    pub fn insert(&mut self, id: EntityId, data: T) {
        self.components.insert(id, data);
    }
}

impl<T: Component> ComponentRegistry<T> {
    pub fn iter<'a>(&'a self, entities: &'a Vec<EntityId>) -> impl Iterator<Item = (EntityId, &T)> {
        self.components.iter().filter_map(|(id, component)| {
            if entities.contains(id) {
                Some((*id, component))
            } else {
                None
            }
        })
    }

    pub fn iter_mut<'a>(
        &'a mut self,
        entities: &'a Vec<EntityId>,
    ) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.components.iter_mut().filter_map(|(id, component)| {
            if entities.contains(id) {
                Some((*id, component))
            } else {
                None
            }
        })
    }
}

impl<T: Component> Registry for ComponentRegistry<T> {
    fn contains(&self, id: &EntityId) -> bool {
        self.components.contains_key(id)
    }

    fn remove(&mut self, id: &EntityId) {
        self.components.remove(id);
    }

    fn clear(&mut self) {
        self.components.clear()
    }

    fn update(&mut self) {
        self.components.retain(|id, _| !self.destroyed.contains(id));
        self.created.clear();
        self.enabled.clear();
        self.disabled.clear();
        self.destroyed.clear();
    }

    fn enable(&mut self, id: &EntityId) {
        if !self.destroyed.contains(id) {
            self.enabled.insert(*id);
            self.disabled.remove(id);
        }
    }

    fn disable(&mut self, id: &EntityId) {
        if !self.destroyed.contains(id) {
            self.disabled.insert(*id);
            self.enabled.remove(id);
        }
    }

    fn destroy(&mut self, id: &EntityId) {
        if !self.destroyed.contains(id) {
            self.destroyed.insert(*id);
            self.enabled.remove(id);
            self.disabled.remove(id);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
