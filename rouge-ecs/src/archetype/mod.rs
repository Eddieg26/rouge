use crate::{
    core::{ComponentId, Entity},
    storage::{
        sparse::{SparseMap, SparseSet},
        table::TableId,
    },
};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    pub fn new(components: &[ComponentId]) -> Self {
        let mut components = components.iter().copied().collect::<Vec<_>>();
        components.sort();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        components.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl std::ops::Deref for ArchetypeId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<TableId> for ArchetypeId {
    fn into(self) -> TableId {
        TableId::from(self.0)
    }
}

impl Into<TableId> for &ArchetypeId {
    fn into(self) -> TableId {
        TableId::from(self.0)
    }
}

pub struct Archetype {
    id: ArchetypeId,
    entities: SparseMap<usize, Entity>,
    components: Box<[ComponentId]>,
}

impl Archetype {
    pub fn new(id: ArchetypeId, components: Vec<ComponentId>) -> Self {
        Self {
            id,
            entities: SparseMap::new(),
            components: components.into_boxed_slice(),
        }
    }

    pub fn added(&self, component: ComponentId) -> Vec<ComponentId> {
        let mut components = self.components.to_vec();
        components.push(component);
        components
    }

    pub fn removed(&self, component: ComponentId) -> Vec<ComponentId> {
        let mut components = self.components.to_vec();
        components.retain(|c| *c != component);
        components
    }

    pub fn id(&self) -> &ArchetypeId {
        &self.id
    }

    pub fn entities(&self) -> &[Entity] {
        self.entities.values()
    }

    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }
}

pub struct Archetypes {
    archetypes: SparseMap<ArchetypeId, Archetype>,
    entities: SparseSet<ArchetypeId>,
    components: SparseMap<ComponentId, HashSet<ArchetypeId>>,
}

impl Archetypes {
    pub fn new() -> Self {
        Self {
            archetypes: SparseMap::new(),
            entities: SparseSet::new(),
            components: SparseMap::new(),
        }
    }

    pub fn archetype_id(&self, entity: Entity) -> Option<&ArchetypeId> {
        self.entities.get(entity.id())
    }

    pub fn archetype(&self, archetype_id: &ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(archetype_id)
    }

    pub fn entity_archetype(&self, entity: Entity) -> Option<&Archetype> {
        self.entities
            .get(entity.id())
            .and_then(|id| self.archetypes.get(id))
    }

    pub fn entities(&self, components: &[ComponentId], without: &[ComponentId]) -> Vec<&Entity> {
        let mut entities = vec![];

        for component_id in components {
            if let Some(archetypes) = self.components.get(component_id) {
                for achetype in archetypes {
                    if let Some(archetype) = self.archetypes.get(achetype) {
                        let has = components
                            .iter()
                            .all(|c| archetype.components().contains(c));
                        if has && without.iter().all(|c| !archetype.components().contains(c)) {
                            entities.extend(archetype.entities());
                        }
                    }
                }
            }
        }

        entities
    }

    pub fn archetypes(
        &self,
        components: &[ComponentId],
        without: &[ComponentId],
    ) -> Vec<&ArchetypeId> {
        let mut results = vec![];

        for component_id in components {
            if let Some(archetypes) = self.components.get(component_id) {
                for achetype in archetypes {
                    if let Some(archetype) = self.archetypes.get(achetype) {
                        let has = components
                            .iter()
                            .all(|c| archetype.components().contains(c));
                        if has && without.iter().all(|c| !archetype.components().contains(c)) {
                            results.push(archetype.id());
                        }
                    }
                }
            }
        }

        results
    }

    pub fn entity_archetypes(
        &self,
        components: &[ComponentId],
        without: &[ComponentId],
        entities: &[Entity],
    ) -> Vec<&ArchetypeId> {
        let mut results = vec![];

        for entity in entities {
            if let Some(archetype) = self.entity_archetype(*entity) {
                let has = components
                    .iter()
                    .all(|c| archetype.components().contains(c));
                if has && without.iter().all(|c| !archetype.components().contains(c)) {
                    results.push(archetype.id());
                }
            }
        }

        results
    }

    pub fn add_entity(&mut self, entity: Entity) -> ArchetypeId {
        let id = ArchetypeId::new(&[]);
        self.entities.insert(entity.id(), id);

        if let Some(archetype) = self.archetypes.get_mut(&id) {
            archetype.entities.insert(entity.id(), entity);
        } else {
            self.archetypes.insert(id, Archetype::new(id, Vec::new()));
            self.archetypes
                .get_mut(&id)
                .unwrap()
                .entities
                .insert(entity.id(), entity);
        }

        id
    }

    pub fn add_component(&mut self, entity: Entity, component: ComponentId) -> Option<ArchetypeId> {
        if let Some(id) = self.entities.get(entity.id()).cloned() {
            let components = {
                let archetype = self.archetypes.get_mut(&id).unwrap();
                archetype.entities.remove(&entity.id());
                archetype.added(component)
            };

            let new_id = ArchetypeId::new(&components);

            for component in components.iter() {
                self.add_component_archetype(*component, new_id);
            }

            if let Some(archetype) = self.archetypes.get_mut(&new_id) {
                archetype.entities.insert(entity.id(), entity);
            } else {
                let mut archetype = Archetype::new(new_id, components);
                archetype.entities.insert(entity.id(), entity);
                self.archetypes.insert(new_id, archetype);
            }

            self.entities.insert(entity.id(), new_id);

            Some(new_id)
        } else {
            None
        }
    }

    pub fn remove_component(
        &mut self,
        entity: Entity,
        component: ComponentId,
    ) -> Option<ArchetypeId> {
        if let Some(id) = self.entities.get(entity.id()).cloned() {
            let components = {
                let archetype = self.archetypes.get_mut(&id).unwrap();
                archetype.entities.remove(&entity.id());
                archetype.removed(component)
            };
            let new_id = ArchetypeId::new(&components);

            for component in components.iter() {
                self.add_component_archetype(*component, new_id);
            }

            if let Some(archetype) = self.archetypes.get_mut(&new_id) {
                archetype.entities.insert(entity.id(), entity);
            } else {
                let mut archetype = Archetype::new(new_id, components);
                archetype.entities.insert(entity.id(), entity);
                self.archetypes.insert(new_id, archetype);
            }

            self.entities.insert(entity.id(), new_id);

            Some(new_id)
        } else {
            None
        }
    }

    pub fn delete_entity(&mut self, entity: Entity) -> Option<ArchetypeId> {
        if let Some(id) = self.entities.remove(entity.id()) {
            let archetype = self.archetypes.get_mut(&id).unwrap();
            archetype.entities.remove(&entity.id());
            Some(id)
        } else {
            None
        }
    }

    pub fn has(&self, entity: Entity, component: ComponentId) -> bool {
        if let Some(id) = self.entities.get(entity.id()) {
            let archetype = self.archetypes.get(id).unwrap();
            archetype.components().contains(&component)
        } else {
            false
        }
    }

    fn add_component_archetype(&mut self, component: ComponentId, id: ArchetypeId) {
        if let Some(archetypes) = self.components.get_mut(&component) {
            archetypes.insert(id);
        } else {
            let mut archetypes = HashSet::new();
            archetypes.insert(id);
            self.components.insert(component, archetypes);
        }
    }
}
