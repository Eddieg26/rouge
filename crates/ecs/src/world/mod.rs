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
    task::TaskPool,
};
use access::WorldAccessTracker;
use action::WorldActions;
use builtin::{
    components::{Children, Parent},
    events::{ComponentUpdate, Despawned, HierarchyUpdate, Spawned},
};
use cell::WorldCell;
use id::WorldId;
use registry::{Metadata, Registry};

pub mod access;
pub mod action;
pub mod builtin;
pub mod cell;
pub mod query;
pub mod registry;
pub mod spawner;

pub struct World {
    id: WorldId,
    access: WorldAccessTracker,
    entities: Entities,
    archetypes: Archetypes,
    registry: Registry,
    events: InvokedEvents,
    actions: WorldActions,
    resources: Resources<true>,
    non_send_resources: Resources<false>,
    configs: SystemConfigs,
    systems: Systems,
    observers: Observers,
    tasks: TaskPool,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            id: WorldId::new(),
            access: WorldAccessTracker::new(),
            entities: Entities::new(),
            archetypes: Archetypes::new(),
            registry: Registry::new(),
            events: InvokedEvents::new(),
            actions: WorldActions::default(),
            resources: Resources::new(),
            non_send_resources: Resources::new(),
            configs: SystemConfigs::new(RunMode::Parallel),
            systems: Systems::new(),
            observers: Observers::new(),
            tasks: TaskPool::default(),
        };

        world.register_builtin();
        world
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn access(&self) -> &WorldAccessTracker {
        &self.access
    }

    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    pub fn entities_mut(&mut self) -> &mut Entities {
        &mut self.entities
    }

    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    pub fn actions(&self) -> &WorldActions {
        &self.actions
    }

    pub fn resources(&self) -> &Resources<true> {
        &self.resources
    }

    pub fn non_send_resources(&self) -> &Resources<false> {
        &self.non_send_resources
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn event_meta(&self, ty: &EventId) -> &Metadata {
        self.registry.get(ty)
    }

    pub fn configs(&self) -> &SystemConfigs {
        &self.configs
    }

    pub fn tasks(&self) -> &TaskPool {
        &self.tasks
    }

    pub fn mode(&self) -> RunMode {
        self.configs.mode()
    }

    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.resources.get::<R>()
    }

    pub fn resource_mut<R: Resource + Send>(&mut self) -> &mut R {
        self.resources.get_mut::<R>()
    }

    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.non_send_resources.get::<R>()
    }

    pub fn non_send_resource_mut<R: Resource>(&mut self) -> &mut R {
        self.non_send_resources.get_mut::<R>()
    }

    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.resources.try_get::<R>()
    }

    pub fn try_resource_mut<R: Resource + Send>(&mut self) -> Option<&mut R> {
        self.resources.try_get_mut::<R>()
    }

    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.non_send_resources.try_get::<R>()
    }

    pub fn try_non_send_resource_mut<R: Resource>(&mut self) -> Option<&mut R> {
        self.non_send_resources.try_get_mut::<R>()
    }

    pub fn register<C: Component>(&mut self) -> &mut Self {
        self.archetypes.register_component::<C>();
        self.registry.register_component::<C>();
        self.register_event::<ComponentUpdate<C>>()
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        let invoked = self.events.invoked();
        self.resources.add(Events::<E>::new(invoked.clone()));
        self
    }

    pub fn register_resource<R: Resource + Send>(&mut self) -> &mut Self {
        self.registry.register_resource::<R>();
        self
    }

    pub fn register_non_send_resource<R: Resource>(&mut self) -> &mut Self {
        self.registry.register_resource::<R>();
        self
    }

    pub fn init_resource<R: Resource + Default + Send>(&mut self) -> &mut Self {
        self.resources.add(R::default());
        self.registry.register_resource::<R>();
        self
    }

    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.resources.add(resource);
        self.registry.register_resource::<R>();
        self
    }

    pub fn init_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.non_send_resources.add(R::default());
        self.registry.register_resource::<R>();
        self
    }

    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.non_send_resources.add(resource);
        self.registry.register_resource::<R>();
        self
    }

    pub fn remove_resource<R: Resource + Send>(&mut self) -> Option<R> {
        self.resources.remove::<R>()
    }

    pub fn remove_non_send_resource<R: Resource>(&mut self) -> Option<R> {
        self.non_send_resources.remove::<R>()
    }

    pub fn invoke_event<E: Event>(&mut self, event: E) -> &mut Self {
        self.events.invoke::<E>();
        self.resource_mut::<Events<E>>().add(event);
        self
    }

    pub fn add_systems<M>(
        &mut self,
        phase: impl Phase,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.configs
            .add_systems(Type::of::<Global>(), phase, systems);
        self
    }

    pub fn observe<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) -> &mut Self {
        self.observers.add_observers::<E, M>(observers);
        self
    }

    pub fn add_phase<P: Phase>(&mut self) -> &mut Self {
        self.systems.schedule_mut().add_child(P::schedule());
        self
    }

    pub fn add_sub_phase<Main: Phase, Sub: Phase>(&mut self) -> &mut Self {
        self.systems.schedule_mut().add_sub_child::<Main, Sub>();
        self
    }

    pub fn add_phase_before<P: Phase, Before: Phase>(&mut self) -> &mut Self {
        self.systems.schedule_mut().insert_before::<P, Before>();
        self
    }

    pub fn add_phase_after<P: Phase, After: Phase>(&mut self) -> &mut Self {
        self.systems.schedule_mut().insert_after::<P, After>();
        self
    }

    pub fn run(&mut self, phase: impl Phase) {
        if !self.configs.is_empty() {
            self.systems.add_graphs(self.configs.build_graphs());
        }

        self.systems.run(phase, WorldCell::from(self as &Self));
    }

    pub fn flush(&mut self, phase: Option<PhaseId>) {
        self.actions.drain().drain(..).for_each(|a| a.execute(self));

        let invoked = {
            let invoked = self.events.invoked();
            let mut invoked = invoked.lock().unwrap();
            let mut invoked = invoked.drain(..).collect::<Vec<_>>();
            if let Some(phases) = phase.and_then(|p| self.events.deferred(p)) {
                invoked.extend(phases);
            }

            invoked
        };

        self.observers.build(self.mode());
        self.observers.run(WorldCell::from(self as &Self), invoked);
    }

    pub fn flush_type(&mut self, ty: EventId) {
        let invoked = self.events.invoked();
        let mut invoked = invoked.lock().unwrap();
        if invoked.shift_remove(&ty) {
            self.observers.build(self.mode());
            self.observers.run(WorldCell::from(self as &Self), vec![ty]);
        }
    }

    fn register_builtin(&mut self) {
        self.register::<Parent>();
        self.register::<Children>();
        self.register_event::<Spawned>();
        self.register_event::<Despawned>();
        self.register_event::<HierarchyUpdate>();
    }
}

impl World {
    pub fn spawner(&mut self) -> spawner::Spawner {
        spawner::Spawner::new(self)
    }

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

pub mod id {
    use std::sync::atomic::AtomicU32;

    pub static mut WORLD_ID: AtomicU32 = AtomicU32::new(0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct WorldId(u32);

    impl WorldId {
        pub fn new() -> Self {
            let id = unsafe { WORLD_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) };
            Self(id)
        }
    }
    impl std::ops::Deref for WorldId {
        type Target = u32;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
