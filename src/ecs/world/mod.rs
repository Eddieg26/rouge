use super::{
    archetype::ArchetypeManager,
    component::{manager::ComponentManager, registry::ComponentRegistry, Component},
    entity::registry::EntityRegistry,
    resource::{manager::ResourceManager, Resource},
    state::StateManager,
    EntityId, ResetInterval, State,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub use events::*;
pub use query::*;

pub mod events;
pub mod query;

type Entities = Rc<RefCell<EntityRegistry>>;
type Archetypes = Rc<RefCell<ArchetypeManager>>;

pub struct World {
    entities: Entities,
    archetypes: Archetypes,
    components: ComponentManager,
    resources: ResourceManager,
    states: StateManager,
}

impl World {
    pub fn new() -> World {
        let entities = Rc::new(RefCell::new(EntityRegistry::new()));
        let archetypes = Rc::new(RefCell::new(ArchetypeManager::new()));

        World {
            components: ComponentManager::new(),
            resources: ResourceManager::new(),
            entities,
            archetypes,
            states: StateManager::new(),
        }
    }

    pub fn component_manager(&self) -> &ComponentManager {
        &self.components
    }

    pub fn components<T: Component>(&self) -> Ref<'_, ComponentRegistry<T>> {
        self.components.registry::<T>()
    }

    pub fn components_mut<T: Component>(&self) -> RefMut<'_, ComponentRegistry<T>> {
        self.components.registry_mut::<T>()
    }

    pub fn resource<T: Resource>(&self) -> Ref<'_, T> {
        self.resources.resource::<T>()
    }

    pub fn resource_mut<T: Resource>(&self) -> RefMut<'_, T> {
        self.resources.resource_mut::<T>()
    }

    pub fn entities(&self) -> Ref<'_, EntityRegistry> {
        self.entities.borrow()
    }

    pub fn entities_mut(&self) -> RefMut<'_, EntityRegistry> {
        self.entities.borrow_mut()
    }

    pub fn archetypes(&self) -> Ref<'_, ArchetypeManager> {
        self.archetypes.borrow()
    }

    pub fn archetypes_mut(&self) -> RefMut<'_, ArchetypeManager> {
        self.archetypes.borrow_mut()
    }

    pub fn state<T: State>(&self) -> Ref<'_, T> {
        self.states.get::<T>()
    }

    pub fn state_mut<T: State>(&self) -> RefMut<'_, T> {
        self.states.get_mut::<T>()
    }
}

impl World {
    pub fn register<T: Component>(&mut self) {
        self.components.register::<T>();
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources.register(resource);
    }

    pub fn insert_state<T: State>(&mut self, state: T, interval: ResetInterval) {
        self.states.register(state, interval);
    }
}

impl World {
    pub fn add_event<T: Event>(&self, event: T) {
        self.resource_mut::<EventManager>().add(event);
    }

    pub fn spawn(&self, entity: CreateEntity) {
        self.resource_mut::<EventManager>().add(entity);
    }

    pub fn spawn_empty(&self) -> EntityId {
        let entity = CreateEntity::new();
        let id = entity.id().clone();
        self.resource_mut::<EventManager>().add(entity);
        id
    }

    pub fn destroy(&self, id: &EntityId) {
        let event = DestroyEntity::new(*id);
        self.resource_mut::<EventManager>().add(event);
    }

    pub fn remove<T: Component>(&self, _id: &EntityId) {
        let event = RemoveComponent::<T>::new(*_id);
        self.resource_mut::<EventManager>().add(event);
    }

    pub fn has<T: Component>(&self, id: &EntityId) -> bool {
        self.components::<T>().get(id).is_some()
    }
}
