use self::{
    lifecycle::Lifecycle,
    meta::ComponentActionMeta,
    resource::{Resource, Resources},
};
use crate::{
    archetype::Archetypes,
    core::{Component, ComponentId, Components, Entities, Entity},
    schedule::{GlobalSchedules, SceneSchedules, Schedule, ScheduleLabel, SchedulePhase},
    storage::table::Tables,
    system::{
        observer::{
            action::{Action, ActionOutputs, Actions},
            Observables, Observers,
        },
        IntoSystem,
    },
};

pub mod lifecycle;
pub mod meta;
pub mod query;
pub mod resource;

pub struct World {
    resources: Resources,
    archetypes: Archetypes,
    entities: Entities,
    components: Components,
    tables: Tables<Entity>,
}

impl World {
    pub fn new() -> Self {
        let mut resources = Resources::new();
        resources.insert(GlobalSchedules::new());
        resources.insert(SceneSchedules::new());
        resources.insert(Observables::new());
        resources.insert(ActionOutputs::new());
        resources.insert(Actions::new());

        Self {
            resources,
            archetypes: Archetypes::new(),
            entities: Entities::new(),
            components: Components::new(),
            tables: Tables::new(),
        }
    }

    pub fn register<C: Component>(&mut self) {
        let id = self.components.register::<C>();
        self.components
            .extend_meta(id, ComponentActionMeta::new::<C>());
    }

    pub fn add_resource<T: Resource>(&mut self, resource: T) {
        self.resources.insert(resource);
    }

    pub fn add_system<M>(
        &mut self,
        phase: impl SchedulePhase,
        label: impl ScheduleLabel,
        system: impl IntoSystem<M>,
    ) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.add_system(phase, label, system);
    }

    pub fn add_schedule(
        &mut self,
        phase: impl SchedulePhase,
        label: impl ScheduleLabel,
        schedule: Schedule,
    ) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.add_schedule(phase, label, schedule);
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

    pub fn resource<R: Resource>(&self) -> &R {
        self.resources.get::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> &mut R {
        self.resources.get_mut::<R>()
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
                        (meta.on_remove())(&entity, self.resources.get_mut::<ActionOutputs>());
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

    pub fn run<P: SchedulePhase>(&mut self) {
        let schedules = self.resources.get::<GlobalSchedules>();
        schedules.run::<P>(self);

        let schedules = self.resources.get::<SceneSchedules>();
        schedules.run::<P>(self);

        self.flush();
    }

    fn flush(&mut self) {
        if self.resources.get::<Actions>().is_empty() {
            return;
        }

        let outputs = {
            let mut actions = std::mem::take(self.resources.get_mut::<Actions>());
            let mut outputs = actions.execute(self);
            let action_outputs = self.resources.get_mut::<ActionOutputs>().take();
            self.resources.get_mut::<Actions>().append(actions);

            outputs.merge(action_outputs);
            outputs
        };

        let mut observers = std::mem::take(self.resources.get_mut::<Observables>());
        observers.execute(outputs, self);
        self.resources.get_mut::<Observables>().swap(observers);

        self.flush();
    }

    pub fn init(&mut self) {
        let schedules = self.resources.get_mut::<GlobalSchedules>();
        schedules.build();

        let schedules = self.resources.get_mut::<SceneSchedules>();
        schedules.build();
    }
}
