use crate::{
    core::{
        component::Component,
        entity::Entities,
        registry::Type,
        resource::{Resource, Resources},
    },
    event::{Event, EventId, EventMeta, EventRegistry, Events},
    system::{
        observer::Observers,
        schedule::{Phase, PhaseId},
        systems::{Global, RunMode, SystemConfigs, SystemMeta, Systems},
        IntoSystemConfigs,
    },
};
use action::WorldActions;
use components::{ComponentId, ComponentMeta, Components};
use id::WorldId;

pub mod action;
pub mod components;
pub mod id;

pub struct World {
    id: WorldId,
    entities: Entities,
    components: Components,
    events: EventRegistry,
    actions: WorldActions,
    resources: Resources<true>,
    non_send_resources: Resources<false>,
    system_meta: SystemMeta,
    system_configs: SystemConfigs,
    systems: Option<Systems>,
    observers: Observers,
}

impl World {
    pub fn new() -> Self {
        Self {
            id: WorldId::new(),
            entities: Entities::new(),
            components: Components::new(),
            events: EventRegistry::new(),
            actions: WorldActions::default(),
            resources: Resources::new(),
            non_send_resources: Resources::new(),
            system_meta: SystemMeta::new(RunMode::Parallel),
            system_configs: SystemConfigs::new(),
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
    pub fn components(&self) -> &Components {
        &self.components
    }

    #[inline]
    pub fn events(&self) -> &EventRegistry {
        &self.events
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
    pub fn component_meta(&self, ty: &ComponentId) -> Option<&ComponentMeta> {
        self.components.get(ty)
    }

    #[inline]
    pub fn event_meta(&self, ty: &EventId) -> EventMeta {
        self.events
            .get(ty)
            .copied()
            .expect(&format!("EventMeta not found for {:?}", ty))
    }

    #[inline]
    pub fn system_meta(&self) -> &SystemMeta {
        &self.system_meta
    }

    #[inline]
    pub fn resource<R: Resource + Send>(&self) -> &R {
        self.resources.get::<R>()
    }

    #[inline]
    pub fn resource_mut<R: Resource + Send>(&self) -> &mut R {
        self.resources.get_mut::<R>()
    }

    #[inline]
    pub fn non_send_resource<R: Resource>(&self) -> &R {
        self.non_send_resources.get::<R>()
    }

    #[inline]
    pub fn non_send_resource_mut<R: Resource>(&self) -> &mut R {
        self.non_send_resources.get_mut::<R>()
    }

    #[inline]
    pub fn try_resource<R: Resource + Send>(&self) -> Option<&R> {
        self.resources.try_get::<R>()
    }

    #[inline]
    pub fn try_resource_mut<R: Resource + Send>(&self) -> Option<&mut R> {
        self.resources.try_get_mut::<R>()
    }

    #[inline]
    pub fn try_non_send_resource<R: Resource>(&self) -> Option<&R> {
        self.non_send_resources.try_get::<R>()
    }

    #[inline]
    pub fn try_non_send_resource_mut<R: Resource>(&self) -> Option<&mut R> {
        self.non_send_resources.try_get_mut::<R>()
    }

    #[inline]
    pub fn register<C: Component>(&mut self) -> &mut Self {
        let ty = ComponentId::of::<C>();
        self.components.register(ty, ComponentMeta::new::<C>());
        self
    }

    #[inline]
    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        let events = self.events.register::<E>();
        self.resources.add(events);
        self
    }

    #[inline]
    pub fn register_resource<R: Resource + Default + Send>(&mut self) -> &mut Self {
        self.resources.add(R::default());
        self
    }

    #[inline]
    pub fn register_non_send_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.non_send_resources.add(R::default());
        self
    }

    #[inline]
    pub fn add_resource<R: Resource + Send>(&mut self, resource: R) -> &mut Self {
        self.resources.add(resource);
        self
    }

    #[inline]
    pub fn add_non_send_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.non_send_resources.add(resource);
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
        self.system_configs
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

        systems.update(self.system_meta.mode(), &mut self.system_configs);
        systems.run(phase, self);

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

        self.observers.build(self.system_meta.mode());
        self.observers.run(&self, &self.system_meta, invoked);
    }
}
