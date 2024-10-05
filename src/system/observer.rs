use super::{
    schedule::SystemGraph,
    systems::{RunMode, SystemMeta},
    IntoSystemConfigs, SystemConfig,
};
use crate::{
    event::{Event, EventId},
    world::{cell::WorldCell, registry::EventHooks, World},
};
use hashbrown::HashMap;
use indexmap::IndexMap;

pub struct Observer<E: Event> {
    config: SystemConfig,
    _marker: std::marker::PhantomData<E>,
}

impl<E: Event> From<SystemConfig> for Observer<E> {
    fn from(config: SystemConfig) -> Self {
        Self {
            config,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<E: Event> IntoSystemConfigs<()> for Observer<E> {
    fn configs(self) -> Vec<SystemConfig> {
        vec![self.config]
    }

    fn before<Marker>(self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        system.after(self)
    }

    fn after<Marker>(mut self, system: impl IntoSystemConfigs<Marker>) -> Vec<SystemConfig> {
        let mut configs = system.configs();
        let id = configs.first().unwrap().id();

        self.config.after = Some(id);

        match configs.iter().position(|config| config.id() == id) {
            Some(index) => configs.insert(index + 1, self.config),
            None => configs.push(self.config),
        }

        configs
    }
}

pub struct ObserverConfigs {
    configs: HashMap<EventId, Vec<SystemConfig>>,
}

impl ObserverConfigs {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn add<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) {
        let ty = EventId::of::<E>();
        self.configs
            .entry(ty)
            .or_default()
            .extend(observers.configs());
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    pub fn into_observers(&mut self, mode: RunMode) -> IndexMap<EventId, SystemGraph> {
        self.configs
            .drain()
            .map(|(ty, configs)| (ty, SystemGraph::new(mode, configs)))
            .collect()
    }
}

pub struct Observers {
    configs: ObserverConfigs,
    observers: IndexMap<EventId, SystemGraph>,
}

impl Observers {
    pub fn new() -> Self {
        Self {
            configs: ObserverConfigs::new(),
            observers: IndexMap::new(),
        }
    }

    pub fn run(
        &self,
        world: &World,
        meta: &SystemMeta,
        invoked: impl IntoIterator<Item = EventId>,
    ) {
        let world = WorldCell::from(world);
        for ty in invoked {
            if let Some(observers) = self.observers.get(&ty) {
                let event_meta = world.get().event_meta(&ty);
                meta.runner().run(&world, &[observers]);
                event_meta.hooks_as::<EventHooks>().clear(world.get_mut());
            }
        }
    }

    pub fn add_observers<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) {
        self.configs.add::<E, M>(observers);
    }

    pub fn build(&mut self, mode: RunMode) {
        if !self.configs.is_empty() {
            self.observers.extend(self.configs.into_observers(mode));
        }
    }
}
