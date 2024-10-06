use super::{schedule::SystemGraph, systems::RunMode, IntoSystemConfigs, SystemConfig};
use crate::{
    event::{Event, EventId, Events},
    world::{cell::WorldCell, World},
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

pub struct EventExtension {
    clear: fn(&mut World),
}

impl EventExtension {
    pub fn new<E: Event>() -> Self {
        Self {
            clear: |world| {
                world.resource_mut::<Events<E>>().clear();
            },
        }
    }

    pub fn clear(&self, world: &mut World) {
        (self.clear)(world);
    }
}

pub struct ObserverConfigs {
    configs: HashMap<EventId, Vec<SystemConfig>>,
    extensions: HashMap<EventId, EventExtension>,
}

impl ObserverConfigs {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            extensions: HashMap::new(),
        }
    }

    pub fn add<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) {
        let ty = EventId::of::<E>();
        if let Some(configs) = self.configs.get_mut(&ty) {
            configs.extend(observers.configs());
        } else {
            self.configs.insert(ty, observers.configs());
            self.extensions.insert(ty, EventExtension::new::<E>());
        }
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
    extensions: HashMap<EventId, EventExtension>,
}

impl Observers {
    pub fn new() -> Self {
        Self {
            configs: ObserverConfigs::new(),
            observers: IndexMap::new(),
            extensions: HashMap::new(),
        }
    }

    pub fn run(&self, world: WorldCell, invoked: impl IntoIterator<Item = EventId>) {
        let meta = world.get().configs().meta();
        for ty in invoked {
            if let Some(observers) = self.observers.get(&ty) {
                meta.runner().run(&world, &[observers]);
                self.extensions.get(&ty).unwrap().clear(world.get_mut());
            }
        }
    }

    pub fn add_observers<E: Event, M>(&mut self, observers: impl IntoSystemConfigs<M>) {
        self.configs.add::<E, M>(observers);
    }

    pub fn build(&mut self, mode: RunMode) {
        if !self.configs.is_empty() {
            self.observers.extend(self.configs.into_observers(mode));
            let ext = std::mem::take(&mut self.extensions);
            self.extensions.extend(ext);
        }
    }
}
