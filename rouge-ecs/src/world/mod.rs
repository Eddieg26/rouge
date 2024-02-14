use std::sync::{Arc, Mutex};

use self::{
    lifecycle::Lifecycle,
    meta::ComponentActionMeta,
    resource::{LocalResource, LocalResources, Resource, Resources},
};
use crate::{
    archetype::Archetypes,
    core::{Component, ComponentId, Components, Entities, Entity},
    observer::{graph::Observables, Observer},
    process::StartProcess,
    runner::RunMode,
    schedule::{GlobalSchedules, LocalSchedules, Schedule, SchedulePhase},
    storage::table::Tables,
    system::{
        observer::{
            action::{Action, ActionOutputs, Actions},
            builtin::{
                AddChildren, AddComponent, CreateEntity, DeleteEntity, HierarchyChange,
                RemoveChildren, RemoveComponent, SetParent,
            },
            ActionReflectors, Observers,
        },
        IntoSystem,
    },
    GenId, IdAllocator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldId {
    id: usize,
    gen: u32,
    is_main: bool,
}

impl WorldId {
    pub fn new(id: usize, gen: u32, is_main: bool) -> Self {
        Self { id, gen, is_main }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn gen(&self) -> u32 {
        self.gen
    }

    pub fn is_main(&self) -> bool {
        self.is_main
    }
}

impl Into<GenId> for WorldId {
    fn into(self) -> GenId {
        GenId::new(self.id, self.gen)
    }
}

#[derive(Debug, Clone)]
pub struct WorldIdAllocator {
    allocator: Arc<Mutex<IdAllocator>>,
}

impl WorldIdAllocator {
    pub fn new() -> Self {
        Self {
            allocator: Arc::new(Mutex::new(IdAllocator::new())),
        }
    }

    pub fn allocate(&mut self, main: bool) -> WorldId {
        let id = self.allocator.lock().unwrap().allocate();
        WorldId::new(id.id(), id.generation(), main)
    }

    pub fn free(&mut self, id: WorldId) {
        self.allocator.lock().unwrap().free(id.into());
    }
}

impl Resource for WorldIdAllocator {}

pub mod lifecycle;
pub mod meta;
pub mod query;
pub mod resource;

pub struct World {
    id: WorldId,
    resources: Resources,
    local_resources: LocalResources,
    archetypes: Archetypes,
    entities: Entities,
    components: Components,
    tables: Tables<Entity>,
    actions: Actions,
}

impl World {
    pub fn new() -> Self {
        let mut resources = Resources::new();
        resources.insert(GlobalSchedules::new());
        resources.insert(LocalSchedules::new());
        resources.insert(Observables::new(RunMode::Parallel));
        resources.insert(ActionOutputs::new());
        resources.insert(ActionReflectors::new());

        let mut allocator = WorldIdAllocator::new();
        let id = allocator.allocate(true);

        resources.insert(allocator);

        Self {
            id,
            resources,
            local_resources: LocalResources::new(),
            archetypes: Archetypes::new(),
            entities: Entities::new(),
            components: Components::new(),
            tables: Tables::new(),
            actions: Actions::new(),
        }
    }

    pub fn new_sub(main: &World) -> Self {
        let mut resources = Resources::new();
        resources.insert(GlobalSchedules::new());
        resources.insert(LocalSchedules::new());
        resources.insert(Observables::new(RunMode::Parallel));
        resources.insert(ActionOutputs::new());
        resources.insert(main.resource::<ActionReflectors>().clone());

        let mut allocator = main.resource::<WorldIdAllocator>().clone();
        let id = allocator.allocate(false);
        resources.insert(allocator);

        Self {
            id,
            resources,
            local_resources: LocalResources::new(),
            archetypes: Archetypes::new(),
            entities: Entities::new(),
            components: main.components.clone(),
            tables: Tables::new(),
            actions: Actions::new(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn register<C: Component>(&mut self) {
        let id = self.components.register::<C>();
        self.components
            .extend_meta(id, ComponentActionMeta::new::<C>());
        self.register_action::<AddComponent<C>>();
        self.register_action::<RemoveComponent<C>>();
    }

    pub fn register_action<A: Action>(&mut self) {
        let reflectors = self.resource_mut::<ActionReflectors>();
        reflectors.register::<A>();
    }

    pub fn add_resource<R: Resource>(&mut self, resource: R) {
        self.resources.insert(resource);
    }

    pub fn add_local_resource<R: LocalResource>(&mut self, resource: R) {
        self.local_resources.insert(resource);
    }

    pub fn add_system<M>(&mut self, phase: impl SchedulePhase, system: impl IntoSystem<M>) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.add_system(phase, system);
    }

    pub fn add_schedule(&mut self, phase: impl SchedulePhase, schedule: Schedule) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.add_schedule(phase, schedule);
    }

    pub fn add_observer<A: Action>(&mut self, observer: Observer<A>) {
        self.resources
            .get_mut::<Observables>()
            .add_observer(observer);
    }

    pub fn add_observers<A: Action>(&mut self, observers: Observers<A>) {
        self.resources
            .get_mut::<Observables>()
            .add_observers(observers);
    }

    pub fn component_id<C: Component>(&self) -> ComponentId {
        self.components.id::<C>()
    }

    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    pub fn components(&self) -> &Components {
        &self.components
    }

    pub fn tables(&self) -> &Tables<Entity> {
        &self.tables
    }

    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut Actions {
        &mut self.actions
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.resources.get::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.resources.get_mut::<R>()
    }

    pub fn remove_resource<R: Resource>(&mut self) -> R {
        self.resources.remove::<R>()
    }

    pub fn has_resource<R: Resource>(&self) -> bool {
        self.resources.contains::<R>()
    }

    pub fn try_resource<R: Resource>(&self) -> Option<&R> {
        self.resources.try_get::<R>()
    }

    pub fn try_resource_mut<R: Resource>(&self) -> Option<&mut R> {
        self.resources.try_get_mut::<R>()
    }

    pub fn try_remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.resources.try_remove::<R>()
    }

    pub fn local_resource<R: LocalResource>(&self) -> &R {
        self.local_resources.get::<R>()
    }

    pub fn local_resource_mut<R: LocalResource>(&self) -> &mut R {
        self.local_resources.get_mut::<R>()
    }

    pub fn remove_local_resource<R: LocalResource>(&mut self) -> R {
        self.local_resources.remove::<R>()
    }

    pub fn has_local_resource<R: LocalResource>(&self) -> bool {
        self.local_resources.contains::<R>()
    }

    pub fn try_local_resource<R: LocalResource>(&self) -> Option<&R> {
        self.local_resources.try_get::<R>()
    }

    pub fn try_local_resource_mut<R: LocalResource>(&self) -> Option<&mut R> {
        self.local_resources.try_get_mut::<R>()
    }

    pub fn try_remove_local_resource<R: LocalResource>(&mut self) -> Option<R> {
        self.local_resources.try_remove::<R>()
    }

    pub fn create(&mut self) -> Entity {
        let entity = self.entities.create();
        Lifecycle::create_entity(entity, &mut self.archetypes, &mut self.tables);
        entity
    }

    pub fn has<C: Component>(&self, entity: Entity) -> bool {
        let component_id = self.components.id::<C>();
        self.archetypes.has(entity, component_id)
    }

    pub fn component<C: Component>(&self, entity: Entity) -> Option<&C> {
        let component_id = self.components.id::<C>();
        let archetype = self.archetypes.archetype_id(entity)?;
        let table = self.tables.get((*archetype).into())?;

        table.get::<C>(entity, component_id.into())
    }

    pub fn component_mut<C: Component>(&self, entity: Entity) -> Option<&mut C> {
        let component_id = self.components.id::<C>();
        let archetype = self.archetypes.archetype_id(entity)?;
        let table = self.tables.get((*archetype).into())?;

        table.get_mut::<C>(entity, component_id.into())
    }

    pub fn add_component<C: Component>(&mut self, entity: Entity, component: C) {
        let component_id = self.components.id::<C>();
        Lifecycle::add_component(
            entity,
            component_id,
            component,
            &mut self.archetypes,
            &mut self.tables,
        );
    }

    pub fn remove_component<C: Component>(&mut self, entity: Entity) {
        let component_id = self.components.id::<C>();
        Lifecycle::remove_component(entity, component_id, &mut self.archetypes, &mut self.tables);
    }

    pub fn delete(&mut self, entity: Entity) {
        let deleted = self.entities.delete(entity, true);
        for entity in deleted {
            if let Some(row) =
                Lifecycle::delete_entity(entity, &mut self.archetypes, &mut self.tables)
            {
                for column in row.indices() {
                    let id = ComponentId::from(column);

                    if let Some(meta) = self.components.meta(id).extension::<ComponentActionMeta>()
                    {
                        meta.on_remove(&entity, self.resources.get_mut::<ActionOutputs>());
                    }
                }
            }
        }
    }

    pub fn set_parent(&mut self, entity: Entity, parent: Option<Entity>) {
        self.entities.set_parent(entity, parent)
    }

    pub fn add_child(&mut self, entity: Entity, child: Entity) {
        self.entities.add_child(entity, child)
    }

    pub fn remove_child(&mut self, entity: Entity, child: Entity) {
        self.entities.remove_child(entity, child)
    }

    pub fn scoped_resource<R: Resource, T>(
        &mut self,
        callback: impl FnOnce(&mut World, &R) -> T,
    ) -> T {
        let resource = self.resources.remove::<R>();
        let ret = callback(self, &resource);
        self.resources.insert(resource);

        ret
    }

    pub fn run<P: SchedulePhase>(&mut self) {
        let schedules = self.resources.get::<GlobalSchedules>();
        schedules.run::<P>(self);

        let schedules = self.resources.get::<LocalSchedules>();
        schedules.run::<P>(self);

        self.flush();
    }

    pub fn flush(&mut self) {
        while !self.actions.is_empty() {
            let outputs = {
                let reflectors = self.resource::<ActionReflectors>().clone();
                let mut outputs = reflectors.execute(self);
                let action_outputs = self.resource_mut::<ActionOutputs>().take();
                outputs.merge(action_outputs);
                outputs
            };

            self.scoped_resource::<Observables, ()>(|world, observers| {
                observers.run(world, outputs);
            });
        }
    }

    pub fn flush_actions<A: Action>(&mut self) {
        let outputs = {
            let reflectors = self.resource::<ActionReflectors>().clone();
            let mut outputs = reflectors.execute_actions::<A>(self);
            let action_outputs = self.resource_mut::<ActionOutputs>().take();
            outputs.merge(action_outputs);
            outputs
        };

        self.scoped_resource::<Observables, ()>(|world, observers| {
            observers.run(world, outputs);
        });
    }

    pub fn init(&mut self) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.build();

        let schedules = self.resources.get_mut::<LocalSchedules>();
        schedules.build();

        let observers = self.resources.get_mut::<Observables>();
        observers.build();

        self.init_actions();
    }

    fn init_actions(&mut self) {
        self.register_action::<StartProcess>();
        self.register_action::<CreateEntity>();
        self.register_action::<DeleteEntity>();
        self.register_action::<SetParent>();
        self.register_action::<AddChildren>();
        self.register_action::<RemoveChildren>();
        self.register_action::<HierarchyChange>();
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let mut allocator = self.resources.remove::<WorldIdAllocator>();
        allocator.free(self.id);
    }
}
