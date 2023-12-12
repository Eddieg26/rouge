use super::World;
use crate::ecs::{entity::ObjectState, Component, EntityId, Registry, Resource};
use std::{any::TypeId, collections::HashMap};

pub trait Event: std::any::Any + 'static {
    type Data: 'static;
    const PRIORITY: u32 = 1;
    fn execute(&mut self, world: &World) -> Self::Data;
}

pub struct EventPriority;

impl EventPriority {
    pub const CREATE_ENTITY: u32 = u32::MAX;
    pub const DESTROY_ENTITY: u32 = 0;
    pub const ENABLE_ENTITY: u32 = EventPriority::CREATE_ENTITY - 1;
    pub const DISABLE_ENTITY: u32 = EventPriority::CREATE_ENTITY - 1;
    pub const ADD_COMPONENT: u32 = EventPriority::CREATE_ENTITY - 1;
    pub const REMOVE_COMPONENT: u32 = EventPriority::ADD_COMPONENT - 1;
    pub const ENABLE_COMPONENT: u32 = EventPriority::ADD_COMPONENT - 1;
    pub const UPDATE_COMPONENT: u32 = EventPriority::ADD_COMPONENT - 1;
    pub const DISABLE_COMPONENT: u32 = EventPriority::ADD_COMPONENT - 1;
}

pub trait BaseDispatcher {
    fn dispatch(&mut self, world: &World);
    fn clear(&mut self);

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub struct EventDispatcher<T: 'static, E: Event<Data = T>> {
    events: Vec<E>,
    observers: Vec<Box<dyn Fn(&[T], &World)>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static, E: Event<Data = T>> EventDispatcher<T, E> {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            observers: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn add(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn observe<F: Fn(&[T], &World) + 'static>(&mut self, observer: F) {
        self.observers.push(Box::new(observer));
    }
}

impl<T: 'static, E: Event<Data = T>> BaseDispatcher for EventDispatcher<T, E> {
    fn dispatch(&mut self, world: &World) {
        let mut results = Vec::new();

        for event in self.events.iter_mut() {
            results.push(event.execute(world));
        }

        for observer in self.observers.iter() {
            (*observer)(&results, world);
        }

        self.events.clear();
    }

    fn clear(&mut self) {
        self.events.clear();
        self.observers.clear()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct EventManager {
    events: HashMap<TypeId, Box<dyn BaseDispatcher>>,
    priority: HashMap<TypeId, u32>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            priority: HashMap::new(),
        }
    }

    pub fn add<E: Event>(&mut self, event: E) {
        let type_id = TypeId::of::<E>();
        let dispatcher = self
            .events
            .entry(type_id)
            .or_insert_with(|| Box::new(EventDispatcher::<E::Data, E>::new()));

        self.priority.entry(type_id).or_insert_with(|| E::PRIORITY);

        dispatcher
            .as_any_mut()
            .downcast_mut::<EventDispatcher<E::Data, E>>()
            .unwrap()
            .add(event);
    }

    pub fn observe<E: Event>(&mut self, observer: impl Fn(&[E::Data], &World) + 'static) {
        let type_id = TypeId::of::<E>();
        let dispatcher = self
            .events
            .entry(type_id)
            .or_insert_with(|| Box::new(EventDispatcher::<E::Data, E>::new()));

        self.priority.entry(type_id).or_insert_with(|| E::PRIORITY);

        dispatcher
            .as_any_mut()
            .downcast_mut::<EventDispatcher<E::Data, E>>()
            .unwrap()
            .observe(observer);
    }

    pub fn dispatch(&mut self, world: &World) {
        let dispatchers = self.sort_dispatchers();

        for (_, dispatcher) in dispatchers {
            dispatcher.dispatch(world);
        }
    }

    pub fn dispatch_type<T: Event>(&mut self, world: &World) {
        let type_id = TypeId::of::<T>();
        if let Some(dispatcher) = self.events.get_mut(&type_id) {
            dispatcher
                .as_any_mut()
                .downcast_mut::<EventDispatcher<T::Data, T>>()
                .unwrap()
                .dispatch(world);
        }
    }

    pub fn clear(&mut self) {
        for dispatcher in self.events.values_mut() {
            dispatcher.clear();
        }
    }

    fn sort_dispatchers(&mut self) -> Vec<(&TypeId, &mut Box<dyn BaseDispatcher>)> {
        let mut dispatchers = self.events.iter_mut().collect::<Vec<_>>();

        dispatchers.sort_by(|a, b| {
            let a_priority = self.priority.get(a.0).unwrap();
            let b_priority = self.priority.get(b.0).unwrap();

            a_priority.cmp(b_priority)
        });

        dispatchers
    }
}

impl Resource for EventManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct AddComponent<T: Component> {
    entity_id: EntityId,
    component: Option<T>,
}

impl<T: Component> AddComponent<T> {
    pub fn new(entity_id: EntityId, component: T) -> Self {
        Self {
            entity_id,
            component: Some(component),
        }
    }
}

impl<T: Component> Event for AddComponent<T> {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::ADD_COMPONENT;

    fn execute(&mut self, world: &super::World) -> EntityId {
        world
            .components_mut::<T>()
            .insert(self.entity_id, self.component.take().unwrap());
        world.archetypes_mut().add_component::<T>(self.entity_id);
        self.entity_id
    }
}

pub struct CreateEntity {
    entity_id: EntityId,
}

impl CreateEntity {
    pub fn new() -> Self {
        Self {
            entity_id: EntityId::uuid(),
        }
    }

    pub fn id(&self) -> &EntityId {
        &self.entity_id
    }

    pub fn with<T: Component>(self, world: &World, component: T) -> Self {
        let event = AddComponent::<T>::new(self.entity_id, component);
        world.resource_mut::<EventManager>().add(event);

        self
    }
}

impl Event for CreateEntity {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::CREATE_ENTITY;

    fn execute(&mut self, world: &super::World) -> EntityId {
        world.entities.borrow_mut().insert(self.entity_id);

        self.entity_id
    }
}

pub struct DestroyEntity {
    entity_id: EntityId,
}

impl DestroyEntity {
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }
}

impl Event for DestroyEntity {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::DESTROY_ENTITY;

    fn execute(&mut self, world: &super::World) -> EntityId {
        world.entities_mut().destroy(&self.entity_id);
        world.component_manager().destroy(&self.entity_id);
        world.archetypes_mut().destroy_entity(self.entity_id);
        self.entity_id
    }
}

pub struct RemoveComponent<T: Component> {
    entity_id: EntityId,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> RemoveComponent<T> {
    pub fn new(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Component> Event for RemoveComponent<T> {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::REMOVE_COMPONENT;

    fn execute(&mut self, world: &super::World) -> EntityId {
        world.components_mut::<T>().destroy(&self.entity_id);
        world.archetypes_mut().remove_component::<T>(self.entity_id);
        self.entity_id
    }
}

pub struct UpdateComponent<T: Component> {
    entity_id: EntityId,
    update: Box<dyn Fn(&mut T)>,
}

impl<T: Component> UpdateComponent<T> {
    pub fn new(entity_id: EntityId, update: impl Fn(&mut T) + 'static) -> Self {
        Self {
            entity_id,
            update: Box::new(update),
        }
    }
}

impl<T: Component> Event for UpdateComponent<T> {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::UPDATE_COMPONENT;

    fn execute(&mut self, world: &super::World) -> EntityId {
        if let Some(mut component) = world.components_mut::<T>().get_mut(&self.entity_id) {
            (self.update)(&mut component);
        }
        self.entity_id
    }
}

pub struct UpdateEntityState {
    entity_id: EntityId,
    state: ObjectState,
}

impl UpdateEntityState {
    pub fn new(entity_id: EntityId, state: ObjectState) -> Self {
        Self { entity_id, state }
    }
}

impl Event for UpdateEntityState {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::ENABLE_ENTITY;

    fn execute(&mut self, world: &super::World) -> EntityId {
        match self.state {
            ObjectState::Enabled => world.entities_mut().enable(&self.entity_id),
            ObjectState::Disabled => world.entities_mut().disable(&self.entity_id),
            _ => (),
        };
        self.entity_id
    }
}

pub struct UpdateComponentState<T: Component> {
    entity_id: EntityId,
    state: ObjectState,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> UpdateComponentState<T> {
    pub fn new(entity_id: EntityId, state: ObjectState) -> Self {
        Self {
            entity_id,
            state,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Component> Event for UpdateComponentState<T> {
    type Data = EntityId;
    const PRIORITY: u32 = EventPriority::ENABLE_COMPONENT;

    fn execute(&mut self, world: &super::World) -> EntityId {
        match self.state {
            ObjectState::Enabled => world.components_mut::<T>().enable(&self.entity_id),
            ObjectState::Disabled => world.components_mut::<T>().disable(&self.entity_id),
            _ => (),
        };
        self.entity_id
    }
}

pub struct EnableComponent<T>(std::marker::PhantomData<T>);

impl<T: Component> EnableComponent<T> {
    pub fn new(entity_id: EntityId) -> UpdateComponentState<T> {
        UpdateComponentState::new(entity_id, ObjectState::Enabled)
    }
}

pub struct DisableComponent<T>(std::marker::PhantomData<T>);

impl<T: Component> DisableComponent<T> {
    pub fn new(entity_id: EntityId) -> UpdateComponentState<T> {
        UpdateComponentState::new(entity_id, ObjectState::Disabled)
    }
}
