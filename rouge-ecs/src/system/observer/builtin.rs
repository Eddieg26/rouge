use super::{action::Actions, Action, ActionOutputs};
use crate::{
    core::{Component, Entity},
    world::World,
};
use std::fmt::Debug;

pub struct CreateEntity {
    add_components: Vec<Box<dyn FnMut(Entity, &mut World)>>,
}

impl CreateEntity {
    pub fn new() -> Self {
        Self {
            add_components: Vec::new(),
        }
    }

    pub fn with<C: Component>(mut self, component: C) -> Self {
        let mut component = Box::new(Some(component));
        let add_component = move |entity: Entity, world: &mut World| {
            if let Some(component) = component.take() {
                world
                    .resource_mut::<Actions>()
                    .add(AddComponent::new(entity, component));
            }
        };

        self.add_components.push(Box::new(add_component));

        self
    }
}

impl Debug for CreateEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateEntity").finish()
    }
}

impl Action for CreateEntity {
    type Output = Entity;
    const PRIORITY: u32 = u32::MAX;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        let entity = world.create();

        for add_component in self.add_components.iter_mut() {
            add_component(entity, world);
        }

        entity
    }
}

pub struct AddComponent<C: Component> {
    entity: Entity,
    component: Option<C>,
}

impl<C: Component> AddComponent<C> {
    pub fn new(entity: Entity, component: C) -> Self {
        Self {
            entity,
            component: Some(component),
        }
    }
}

impl<C: Component> Action for AddComponent<C> {
    type Output = Entity;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 1;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        if let Some(component) = self.component.take() {
            world.add_component(self.entity, component);
        }

        self.entity
    }
}

impl<C: Component> Debug for AddComponent<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddComponent")
            .field("entity", &self.entity)
            .finish()
    }
}

pub struct RemoveComponent<C: Component> {
    entity: Entity,
    _marker: std::marker::PhantomData<C>,
}

impl<C: Component> RemoveComponent<C> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<C: Component> Debug for RemoveComponent<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveComponent")
            .field("entity", &self.entity)
            .finish()
    }
}

impl<C: Component> Action for RemoveComponent<C> {
    type Output = Entity;
    const PRIORITY: u32 = AddComponent::<C>::PRIORITY - 1;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        world.remove_component::<C>(self.entity);

        self.entity
    }

    fn skip(&self, world: &World) -> bool {
        !world.has::<C>(self.entity)
    }
}

#[derive(Debug)]
pub struct DeleteEntity {
    entity: Entity,
}

impl DeleteEntity {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

impl Action for DeleteEntity {
    type Output = Entity;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 100;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        world.delete(self.entity);

        self.entity
    }
}

pub struct SetParent {
    entity: Entity,
    parent: Option<Entity>,
}

impl SetParent {
    pub fn new(entity: Entity, parent: Option<Entity>) -> Self {
        Self { entity, parent }
    }
}

impl Action for SetParent {
    type Output = Entity;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 3;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        world.set_parent(self.entity, self.parent);

        world
            .resource_mut::<ActionOutputs>()
            .add::<HierarchyChange>(self.entity);

        self.entity
    }
}

pub struct AddChildren {
    entity: Entity,
    children: Vec<Entity>,
}

impl AddChildren {
    pub fn new(entity: Entity, children: Vec<Entity>) -> Self {
        Self { entity, children }
    }
}

impl Action for AddChildren {
    type Output = Vec<Entity>;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 3;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        for child in self.children.iter() {
            world.add_child(self.entity, *child);
        }

        world
            .resource_mut::<ActionOutputs>()
            .add::<HierarchyChange>(self.entity);

        self.children.clone()
    }
}

pub struct RemoveChildren {
    entity: Entity,
    children: Vec<Entity>,
}

impl RemoveChildren {
    pub fn new(entity: Entity, children: Vec<Entity>) -> Self {
        Self { entity, children }
    }
}

impl Action for RemoveChildren {
    type Output = Entity;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 3;

    fn execute(&mut self, world: &mut crate::world::World) -> Self::Output {
        for child in self.children.iter() {
            world.remove_child(self.entity, *child);
        }

        world
            .resource_mut::<ActionOutputs>()
            .add::<HierarchyChange>(self.entity);

        self.entity
    }
}

pub struct HierarchyChange {
    entity: Entity,
}

impl HierarchyChange {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

impl Action for HierarchyChange {
    type Output = Entity;
    const PRIORITY: u32 = CreateEntity::PRIORITY - 4;

    fn execute(&mut self, _: &mut crate::world::World) -> Self::Output {
        self.entity
    }
}
