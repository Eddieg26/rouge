use super::{EntityId, ObjectState};
use crate::ecs::registry::Registry;
use std::collections::HashMap;

pub struct EntityRegistry {
    entities: HashMap<EntityId, ObjectState>,
}

impl EntityRegistry {
    pub fn new() -> EntityRegistry {
        EntityRegistry {
            entities: HashMap::new(),
        }
    }

    pub fn get(&self, id: &EntityId) -> Option<&ObjectState> {
        match self.entities.get(id) {
            Some(state) => match state {
                ObjectState::Enabled => Some(state),
                _ => None,
            },
            None => None,
        }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&EntityId, &ObjectState)> {
        self.entities.iter()
    }

    pub fn extend(&mut self, iter: impl Iterator<Item = (EntityId, ObjectState)>) {
        self.entities.extend(iter)
    }

    pub fn create(&mut self) -> EntityId {
        let id = EntityId::zero();
        self.entities.insert(id, ObjectState::Created);

        id
    }

    pub fn insert(&mut self, id: EntityId) {
        self.entities.insert(id, ObjectState::Created);
    }
}

impl Registry for EntityRegistry {
    fn contains(&self, id: &EntityId) -> bool {
        self.entities.contains_key(id)
    }

    fn remove(&mut self, id: &EntityId) {
        self.entities.remove(id);
    }

    fn clear(&mut self) {
        self.entities.clear()
    }

    fn update(&mut self) {
        for state in self.entities.values_mut() {
            match state {
                ObjectState::Created => *state = ObjectState::Enabled,
                _ => (),
            }
        }

        self.entities.retain(|_, state| match state {
            ObjectState::Destroyed => false,
            _ => true,
        });
    }

    fn enable(&mut self, id: &EntityId) {
        self.entities.get_mut(id).and_then(|state| match state {
            ObjectState::Created => {
                *state = ObjectState::Enabled;
                Some(())
            }
            _ => None,
        });
    }

    fn disable(&mut self, id: &EntityId) {
        self.entities.get_mut(id).and_then(|state| match state {
            ObjectState::Enabled => {
                *state = ObjectState::Disabled;
                Some(())
            }
            _ => None,
        });
    }

    fn destroy(&mut self, id: &EntityId) {
        self.entities.get_mut(id).and_then(|state| {
            *state = ObjectState::Destroyed;
            Some(())
        });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
