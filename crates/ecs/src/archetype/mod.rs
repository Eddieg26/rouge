use crate::core::{
    bitset::Bitset,
    component::{Component, ComponentId},
    entity::Entity,
};
use hashbrown::HashSet;
use indexmap::{IndexMap, IndexSet};
use std::hash::Hash;
use table::{ColumnCell, Row, Table};

pub mod table;
pub struct Archetypes {
    entities: IndexMap<Entity, ArchetypeId>,
    archetypes: IndexMap<ArchetypeId, Archetype>,
    components: IndexSet<ComponentId>,
    root: ArchetypeId,
}

impl Archetypes {
    pub fn new() -> Self {
        let root = ArchetypeId(0);
        let archetype = Archetype::new(root, Table::builder().build(), Bitset::new());
        let mut archetypes = IndexMap::new();
        archetypes.insert(root, archetype);

        Self {
            entities: IndexMap::new(),
            archetypes,
            components: IndexSet::new(),
            root,
        }
    }

    pub fn root(&self) -> ArchetypeId {
        self.root
    }

    pub fn entity_archetype(&self, entity: Entity) -> Option<&Archetype> {
        self.entities
            .get(&entity)
            .and_then(|id| self.archetypes.get(id))
    }

    pub fn entity_archetype_mut(&mut self, entity: Entity) -> Option<&mut Archetype> {
        self.entities
            .get(&entity)
            .and_then(|id| self.archetypes.get_mut(id))
    }

    pub fn archetype(&self, id: ArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_component<C: Component>(&self, entity: Entity) -> Option<&C> {
        let archetype = self.entity_archetype(entity)?;
        archetype.table.get_component(&entity)
    }

    pub fn get_component_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        let archetype = self.entity_archetype_mut(entity)?;
        archetype.table.get_component_mut(&entity)
    }

    pub fn has_component<C: Component>(&self, entity: Entity) -> bool {
        let index = self.component_index(&ComponentId::of::<C>());
        match self.entity_archetype(entity) {
            Some(archetype) => archetype.has_component(index),
            None => return false,
        }
    }

    pub fn has_components(
        &self,
        entity: Entity,
        components: impl IntoIterator<Item = impl AsRef<ComponentId>>,
    ) -> bool {
        match self.entity_archetype(entity) {
            Some(archetype) => components
                .into_iter()
                .all(|c| archetype.has_component_id(c.as_ref())),
            None => false,
        }
    }

    pub fn component_index(&self, component: &ComponentId) -> usize {
        self.components
            .get_index_of(component)
            .expect(&format!("Component not found: {:?}", component))
    }

    pub fn register_component<C: Component>(&mut self) {
        let id = ComponentId::of::<C>();
        self.components.insert(id);
    }

    pub fn add_entity(&mut self, entity: Entity) {
        let row = Row::new();

        self.add_entity_sorted(entity, self.root, row);
    }

    #[inline]
    pub fn query(&self, ids: &[ComponentId], exclude: &[ComponentId]) -> IndexSet<&Archetype> {
        let mut bits = Bitset::with_capacity(self.components.len());
        for component in ids {
            let index = self.component_index(component);
            bits.set(index);
        }

        let mut set = IndexSet::new();
        for archetype in self.archetypes.values() {
            let exclude = exclude.iter().any(|c| archetype.has_component_id(c));
            if !exclude && bits.contains(archetype.bits()) {
                set.insert(archetype);
            }
        }

        set
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Option<(ArchetypeId, Row)> {
        let id = self.entities.swap_remove(&entity)?;
        let archetype = self.archetypes.get_mut(&id)?;
        let components = archetype.table.remove_entity(&entity)?;

        Some((id, components))
    }

    pub fn add_component<C: Component>(
        &mut self,
        entity: Entity,
        component: C,
    ) -> Option<EntityMove> {
        let (archetype, mut components) = match self.remove_entity(entity) {
            Some(data) => data,
            None => (self.root, Row::new()),
        };
        let id = ComponentId::of::<C>();
        let mut replaced = Row::new();
        let mut added = HashSet::new();

        if let Some(component) = components.add_cell(id, ColumnCell::from(component)) {
            replaced.add_cell(id, component);
        } else {
            added.insert(id);
        }

        components.sort();

        let to = ArchetypeId::from_iter(components.ids());
        let mv = EntityMove::new_added(archetype, to, added, replaced);
        self.add_entity_sorted(entity, to, components);

        Some(mv)
    }

    pub fn add_components(&mut self, entity: Entity, mut components: Row) -> Option<EntityMove> {
        let (archetype, mut row) = match self.remove_entity(entity) {
            Some(data) => data,
            None => (self.root, Row::new()),
        };
        let mut replaced = Row::new();
        let mut added = HashSet::new();

        for (id, component) in components.drain() {
            if let Some(component) = row.add_cell(id, component) {
                replaced.add_cell(id, component);
            } else {
                added.insert(id);
            }
        }

        row.sort();

        let to = ArchetypeId::from_iter(row.ids());
        let mv = EntityMove::new_added(archetype, to, added, replaced);
        self.add_entity_sorted(entity, to, row);

        Some(mv)
    }

    pub fn remove_component<C: Component>(&mut self, entity: Entity) -> Option<EntityMove> {
        let (archetype, mut row) = match self.remove_entity(entity) {
            Some(data) => data,
            None => (self.root, Row::new()),
        };
        let id = ComponentId::of::<C>();
        let component = row.remove_cell(&id)?;
        let mut removed = Row::new();
        removed.add_cell(id, component);

        let to = ArchetypeId::from_iter(row.ids());
        let mv = EntityMove::new_removed(archetype, to, removed);
        self.add_entity_sorted(entity, to, row);

        Some(mv)
    }

    pub fn remove_components(
        &mut self,
        entity: Entity,
        components: impl IntoIterator<Item = impl AsRef<ComponentId>>,
    ) -> Option<EntityMove> {
        let (archetype, mut row) = match self.remove_entity(entity) {
            Some(data) => data,
            None => (self.root, Row::new()),
        };
        let mut removed = Row::new();

        for component in components {
            let id = component.as_ref();
            if let Some(component) = row.remove_cell(id) {
                removed.add_cell(*id, component);
            }
        }

        let to = ArchetypeId::from_iter(row.ids());
        let mv = EntityMove::new_removed(archetype, to, removed);
        self.add_entity_sorted(entity, to, row);

        Some(mv)
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.archetypes.clear();
    }

    #[inline]
    fn add_entity_sorted(&mut self, entity: Entity, id: ArchetypeId, row: Row) {
        if let Some(archetype) = self.archetypes.get_mut(&id) {
            archetype.table.add_entity(entity, row);
        } else {
            let table = row.into_table(entity);
            let mut bits = Bitset::with_capacity(self.components.len());
            for component in table.ids() {
                let index = self.component_index(component);
                bits.set(index);
            }
            let archetype = Archetype::new(id, table, bits);
            self.archetypes.insert(id, archetype);
        }

        self.entities.insert(entity, id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchetypeId(u32);
impl ArchetypeId {
    pub fn from_iter<'a>(value: impl IntoIterator<Item = &'a ComponentId>) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        for component in value {
            component.hash(&mut hasher);
        }

        Self(hasher.finalize())
    }
}

impl From<&ComponentId> for ArchetypeId {
    fn from(component: &ComponentId) -> Self {
        Self(component.value())
    }
}

impl From<&[ComponentId]> for ArchetypeId {
    fn from(components: &[ComponentId]) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        components.hash(&mut hasher);
        Self(hasher.finalize())
    }
}

impl From<&[&ComponentId]> for ArchetypeId {
    fn from(components: &[&ComponentId]) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        components.hash(&mut hasher);
        Self(hasher.finalize())
    }
}

pub struct Archetype {
    id: ArchetypeId,
    table: Table,
    bits: Bitset,
}

impl Archetype {
    pub fn new(id: ArchetypeId, table: Table, bits: Bitset) -> Self {
        Self { id, table, bits }
    }

    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    pub fn table(&self) -> &Table {
        &self.table
    }

    pub fn has_component(&self, component: usize) -> bool {
        self.bits.get(component)
    }

    pub fn has_component_id(&self, component: &ComponentId) -> bool {
        self.table.has_component(component)
    }

    pub fn bits(&self) -> &Bitset {
        &self.bits
    }
}

impl Hash for Archetype {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Archetype {}

pub struct EntityMove {
    pub from: ArchetypeId,
    pub to: ArchetypeId,
    pub added: HashSet<ComponentId>,
    pub removed: Row,
    pub replaced: Row,
}

impl EntityMove {
    pub fn new(
        from: ArchetypeId,
        to: ArchetypeId,
        added: HashSet<ComponentId>,
        removed: Row,
        replaced: Row,
    ) -> Self {
        Self {
            from,
            to,
            added,
            removed,
            replaced,
        }
    }

    pub fn new_added(
        from: ArchetypeId,
        to: ArchetypeId,
        added: HashSet<ComponentId>,
        replaced: Row,
    ) -> Self {
        Self {
            from,
            to,
            added,
            replaced,
            removed: Row::new(),
        }
    }

    pub fn new_removed(from: ArchetypeId, to: ArchetypeId, removed: Row) -> Self {
        Self {
            from,
            to,
            added: HashSet::new(),
            removed,
            replaced: Row::new(),
        }
    }
}
