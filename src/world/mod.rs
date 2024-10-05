use crate::{
    archetype::{table::Row, Archetypes, EntityMove},
    core::{
        component::{Component, ComponentId},
        entity::{Entities, Entity},
        resource::{Resource, Resources},
        Type,
    },
    event::{Event, EventId, Events, InvokedEvents},
    system::{
        observer::Observers,
        schedule::{Phase, PhaseId},
        systems::{Global, RunMode, SystemConfigs, Systems},
        IntoSystemConfigs,
    },
};
use action::WorldActions;
use cell::WorldCell;
use id::WorldId;
use registry::{Metadata, Registry};

pub mod action;
pub mod builtin;
pub mod cell;
pub mod id;
pub mod registry;

pub struct World {
    id: WorldId,
    entities: Entities,
    archetypes: Archetypes,
    registry: Registry,
    events: InvokedEvents,
    actions: WorldActions,
    resources: Resources<true>,
    non_send_resources: Resources<false>,
    configs: SystemConfigs,
    systems: Option<Systems>,
    observers: Observers,
}

impl World {
    pub fn new() -> Self {
        Self {
            id: WorldId::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            registry: Registry::new(),
            events: InvokedEvents::new(),
            actions: WorldActions::default(),
            resources: Resources::new(),
            non_send_resources: Resources::new(),
            configs: SystemConfigs::new(RunMode::Parallel),
            systems: Some(Systems::new()),
            observers: Observers::new(),
        }
    }

    #[inline]
    pub fn id(&self) -> WorldId {
        self.id
    }

    #[inline]
    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    #[inline]
    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    #[inline]
    pub fn actions(&self) -> &WorldActions {
        &self.actions
    }

    #[inline]
    pub fn resources(&self) -> &Resources<true> {
        &self.resources
    }

    #[inline]
    pub fn non_send_resources(&self) -> &Resources<false> {
        &self.non_send_resources
    }

    #[inline]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    #[inline]
    pub fn event_meta(&self, ty: &EventId) -> &Metadata {
        self.registry.get(ty)
    }

    #[inline]
    pub fn configs(&self) -> &SystemConfigs {
        &self.configs
    }

    #[inline]
    pub fn mode(&self) -> RunMode {
        self.configs.mode()
    }

    #[inline]
    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.resources.get::<R>()
    }

    #[inline]
    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.resources.get_mut::<R>()
    }

    #[inline]
    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.non_send_resources.get::<R>()
    }

    #[inline]
    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.non_send_resources.get_mut::<R>()
    }

    #[inline]
    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.resources.try_get::<R>()
    }

    #[inline]
    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.resources.try_get_mut::<R>()
    }

    #[inline]
    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.non_send_resources.try_get::<R>()
    }

    #[inline]
    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.non_send_resources.try_get_mut::<R>()
    }

    #[inline]
    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.archetypes.register_component::<C>();
        self.registry.register_component::<C>();
        self
    }

    #[inline]
    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.registry.register_event::<E>();
        self
    }

    #[inline]
    pub fn register_resource<R: Resource + Default + Send>(&mut self) -> &mut Self {
        self.resources.add(R::default());
        self.registry.register_resource::<R>();
        self
    }

    #[inline]
    pub fn register_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.non_send_resources.add(R::default());
        self.registry.register_resource::<R>();
        self
    }

    #[inline]
    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.resources.add(resource);
        self.registry.register_resource::<R>();
        self
    }

    #[inline]
    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.non_send_resources.add(resource);
        self.registry.register_resource::<R>();
        self
    }

    #[inline]
    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.resources.remove::<R>()
    }

    #[inline]
    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.non_send_resources.remove::<R>()
    }

    #[inline]
    pub fn invoke_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.events.invoke::<E>();
        self.resource_mut::<Events<E>>().add(event);
        self
    }

    #[inline]
    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.configs
            .add_systems(Type::of::<Global>(), phase, systems);
        self
    }

    #[inline]
    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.observers.add_observers::<E, M>(observers);
        self
    }

    #[inline]
    pub fn run(&mut self, phase: impl Phase) {
        let mut systems = self.systems.take().unwrap();

        if !self.configs.is_empty() {
            systems.add_graphs(self.configs.build_graphs());
        }

        systems.run(phase, WorldCell::from(self as &Self));
        self.systems.replace(systems);
    }

    #[inline]
    pub fn flush(&mut self, phase: Option<PhaseId>) {
        self.actions.drain().drain(..).for_each(|a| a.execute(self));

        let invoked = {
            let mut invoked = self.events.invoked();
            let mut invoked = invoked.drain(..).collect::<Vec<_>>();
            if let Some(phases) = phase.and_then(|p| self.events.deferred(p)) {
                invoked.extend(phases);
            }

            invoked
        };

        self.observers.build(self.mode());
        self.observers.run(WorldCell::from(self as &Self), invoked);
    }

    #[inline]
    pub fn flush_type(&mut self, ty: EventId) {
        let mut invoked = self.events.invoked();
        if invoked.shift_remove(&ty) {
            self.observers.build(self.mode());
            self.observers.run(WorldCell::from(self as &Self), vec![ty]);
        }
    }
}

impl World {
    pub fn spawn(&mut self) -> Entity {
        let entity = self.entities.spawn();
        self.archetypes.add_entity(entity);

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> Option<Row> {
        if self.entities.despawn(&entity) {
            self.archetypes.remove_entity(entity).map(|(_, row)| row)
        } else {
            None
        }
    }

    pub fn has_component<C: Component>(&self, entity: Entity) -> bool {
        self.archetypes.has_component::<C>(entity)
    }

    pub fn has_components(
        &self,
        entity: Entity,
        components: impl IntoIterator<Item = impl AsRef<ComponentId>>,
    ) -> bool {
        self.archetypes.has_components(entity, components)
    }

    pub fn get_component<C: Component>(&self, entity: Entity) -> Option<&C> {
        self.archetypes.get_component::<C>(entity)
    }

    pub fn get_component_mut<C: Component>(&mut self, entity: Entity) -> Option<&mut C> {
        self.archetypes.get_component_mut::<C>(entity)
    }

    pub fn add_component<C: Component>(
        &mut self,
        entity: Entity,
        component: C,
    ) -> Option<EntityMove> {
        self.archetypes.add_component(entity, component)
    }

    pub fn add_components(&mut self, entity: Entity, components: Row) -> Option<EntityMove> {
        self.archetypes.add_components(entity, components)
    }

    pub fn remove_component<C: Component>(&mut self, entity: Entity) -> Option<EntityMove> {
        self.archetypes.remove_component::<C>(entity)
    }

    pub fn remove_components(
        &mut self,
        entity: Entity,
        components: impl IntoIterator<Item = impl AsRef<ComponentId>>,
    ) -> Option<EntityMove> {
        self.archetypes.remove_components(entity, components)
    }
}
