use super::{
    component::{Component, ComponentType},
    entity::EntityId,
    hashid::HashId,
};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

pub type ArchetypeId = u64;
pub type Type = Vec<ComponentType>;

pub struct ArchetypeManager {
    archetypes: HashMap<Type, Rc<RefCell<Archetype>>>,
    entity_index: HashMap<EntityId, Rc<RefCell<Archetype>>>,
    component_index: HashMap<ComponentType, HashMap<ArchetypeId, Rc<RefCell<Archetype>>>>,
}

impl ArchetypeManager {
    pub fn new() -> ArchetypeManager {
        ArchetypeManager {
            archetypes: HashMap::new(),
            entity_index: HashMap::new(),
            component_index: HashMap::new(),
        }
    }

    pub fn create_entity(&mut self, entity: EntityId) {
        if let Some(archetype) = self.archetypes.get(&vec![]) {
            self.entity_index.insert(entity, archetype.clone());
        } else {
            let archetype = Rc::new(RefCell::new(Archetype::new()));
            let _type = archetype.borrow()._type.clone();
            self.entity_index.insert(entity, archetype.clone());
            self.archetypes.insert(_type, archetype);
        }
    }

    pub fn add_component<T: Component>(&mut self, entity: EntityId) {
        if let Some(archetype) = self.entity_index.get(&entity).cloned() {
            let add = { archetype.borrow().edge.get_add_node::<T>() };
            let add_type = { archetype.borrow().add_type::<T>() };
            let add = if let Some(add) = add {
                add.clone()
            } else if let Some(add) = self.archetypes.get(&add_type).cloned() {
                archetype.borrow_mut().edge.create_add_node::<T>(&add);
                add.clone()
            } else {
                let add = Archetype::new_type::<T>(&archetype.borrow());
                let add = Rc::new(RefCell::new(add));
                archetype.borrow_mut().edge.create_add_node::<T>(&add);
                self.archetypes.insert(add_type, add.clone());
                add
            };

            self.entity_index.insert(entity, add.clone());

            add.borrow_mut()
                .add::<T>(&mut archetype.borrow_mut(), entity);
        }
    }

    pub fn remove_component<T: Component>(&mut self, entity: EntityId) {
        if let Some(archetype) = self.entity_index.get(&entity).cloned() {
            let remove = { archetype.borrow().edge.get_add_node::<T>() };
            let remove_type = { archetype.borrow().add_type::<T>() };
            let remove = if let Some(remove) = remove {
                remove.clone()
            } else if let Some(remove) = self.archetypes.get(&remove_type) {
                archetype.borrow_mut().edge.create_remove_node::<T>(remove);
                remove.clone()
            } else {
                let remove = Archetype::new_type::<T>(&archetype.borrow());
                let remove = Rc::new(RefCell::new(remove));
                archetype.borrow_mut().edge.create_remove_node::<T>(&remove);
                self.archetypes.insert(remove_type, remove.clone());
                remove
            };

            self.entity_index.insert(entity, remove.clone());

            remove
                .borrow_mut()
                .remove(&mut archetype.borrow_mut(), entity);
        }
    }

    pub fn get_component_entities(&self, _type: &Type) -> Vec<EntityId> {
        let mut types = _type.clone();
        types.sort();

        if let Some(archetype) = self.archetypes.get(&types) {
            Archetype::get_add_edge_entities(&archetype.borrow())
        } else {
            vec![]
        }
    }

    pub fn destroy_entity(&mut self, entity: EntityId) -> Option<Rc<RefCell<Archetype>>> {
        if let Some(archetype) = self.entity_index.get(&entity) {
            archetype.borrow_mut().destroy(entity);
            self.entity_index.remove(&entity)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.archetypes.clear();
        self.entity_index.clear();
        self.component_index.clear();
    }
}

pub struct Edge {
    add: HashMap<ComponentType, Weak<RefCell<Archetype>>>,
    remove: HashMap<ComponentType, Weak<RefCell<Archetype>>>,
}

impl Edge {
    fn new() -> Edge {
        Edge {
            add: HashMap::new(),
            remove: HashMap::new(),
        }
    }

    pub fn get_add_node<T: Component>(&self) -> Option<Rc<RefCell<Archetype>>> {
        let type_id = ComponentType::from(TypeId::of::<T>());
        self.add.get(&type_id)?.upgrade()
    }

    pub fn get_remove_node<T: Component>(&self) -> Option<Rc<RefCell<Archetype>>> {
        let type_id = ComponentType::from(TypeId::of::<T>());
        self.remove.get(&type_id)?.upgrade()
    }

    pub fn create_add_node<T: Component>(&mut self, archetype: &Rc<RefCell<Archetype>>) {
        let type_id = ComponentType::from(TypeId::of::<T>());
        self.add.insert(type_id, Rc::downgrade(&archetype));
    }

    pub fn create_remove_node<T: Component>(&mut self, archetype: &Rc<RefCell<Archetype>>) {
        let type_id = ComponentType::from(TypeId::of::<T>());
        self.add.insert(type_id, Rc::downgrade(&archetype));
    }
}

pub struct Archetype {
    id: u64,
    _type: Type,
    components: HashMap<ComponentType, HashSet<EntityId>>,
    edge: Edge,
}

impl Archetype {
    pub fn new() -> Archetype {
        let id = HashId::new();

        Archetype {
            id,
            _type: vec![],
            components: HashMap::new(),
            edge: Edge::new(),
        }
    }

    pub fn new_type<T: Component>(prev_type: &Archetype) -> Archetype {
        let id: u64 = HashId::new();
        let type_id = ComponentType::from(TypeId::of::<T>());
        let mut types = vec![type_id];
        types.append(&mut (prev_type._type).clone());
        types.sort();

        let mut components = HashMap::new();
        for t in &types {
            components.insert(*t, HashSet::new());
        }

        Archetype {
            id,
            _type: types,
            components,
            edge: Edge::new(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn types(&self) -> &Type {
        &self._type
    }

    pub fn edge(&self) -> &Edge {
        &self.edge
    }

    pub fn edge_mut(&mut self) -> &mut Edge {
        &mut self.edge
    }

    pub fn add<T: Component>(&mut self, src: &mut Archetype, entity: EntityId) {
        let type_id = ComponentType::from(TypeId::of::<T>());
        Archetype::transfer(entity, src, self);
        if let Some(components) = self.components.get_mut(&type_id) {
            components.insert(entity);
        }
    }

    pub fn remove(&mut self, src: &mut Archetype, entity: EntityId) {
        Archetype::transfer(entity, src, self)
    }

    pub fn destroy(&mut self, entity: EntityId) {
        for (_, components) in &mut self.components {
            components.remove(&entity);
        }
    }

    pub fn add_type<T: Component>(&self) -> Type {
        let type_id = ComponentType::from(TypeId::of::<T>());
        let mut add_type = self._type.clone();
        add_type.push(type_id);
        add_type.sort();

        add_type
    }

    pub fn remove_type<T: Component>(&self) -> Type {
        let type_id = ComponentType::from(TypeId::of::<T>());
        let mut remove_type = self._type.clone();
        remove_type.retain(|id| *id != type_id);
        remove_type.sort();

        remove_type
    }

    pub fn transfer(entity: EntityId, src: &mut Archetype, dst: &mut Archetype) {
        for (type_id, components) in &mut src.components {
            if components.remove(&entity) {
                if let Some(components) = dst.components.get_mut(type_id) {
                    components.insert(entity);
                }
            }
        }
    }

    fn get_add_edge_entities(archetype: &Archetype) -> Vec<EntityId> {
        let mut ids = vec![];
        for (_, entities) in &archetype.components {
            ids.extend(entities.iter());
        }

        for (_, edge) in archetype.edge.add.iter() {
            if let Some(edge) = edge.upgrade() {
                ids.extend(Archetype::get_add_edge_entities(&edge.borrow()));
            }
        }

        ids
    }
}
