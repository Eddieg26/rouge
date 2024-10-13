use crate::{config::AssetConfig, library::AssetLibrary};
use ecs::{
    core::resource::Resource,
    event::{Event, Events},
    task::TaskPool,
    world::action::WorldActions,
};
use state::AssetStates;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};

pub mod events;
pub mod state;

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    library: Arc<RwLock<AssetLibrary>>,
    events: Arc<Mutex<AssetEvents>>,
    states: Arc<RwLock<AssetStates>>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> Self {
        Self {
            config: Arc::new(config),
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
            states: Arc::new(RwLock::new(AssetStates::new())),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn states(&self) -> RwLockReadGuard<AssetStates> {
        self.states.read().unwrap()
    }

    pub(crate) fn library_mut(&self) -> std::sync::RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn states_mut(&self) -> std::sync::RwLockWriteGuard<AssetStates> {
        self.states.write().unwrap()
    }

    pub(crate) fn events(&self) -> std::sync::MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }

    pub(crate) fn execute(&self, actions: WorldActions) {
        while let Some(mut event) = self.events().pop() {
            event.execute(self, &actions);
        }

        self.events.lock().unwrap().stop();
    }
}

impl Resource for AssetDatabase {}

pub trait AssetEvent: Send + Sync + 'static {
    fn execute(&mut self, database: &AssetDatabase, actions: &WorldActions);
}

impl<A: AssetEvent> From<A> for Box<dyn AssetEvent> {
    fn from(event: A) -> Self {
        Box::new(event)
    }
}

pub struct AssetEvents {
    events: VecDeque<Box<dyn AssetEvent>>,
    running: bool,
}

impl AssetEvents {
    pub(crate) fn new() -> Self {
        Self {
            events: VecDeque::new(),
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    fn start(&mut self) {
        self.running = true;
    }

    fn stop(&mut self) {
        self.running = false;
    }

    pub fn push(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_back(event.into());
    }

    pub fn push_front(&mut self, event: impl Into<Box<dyn AssetEvent>>) {
        self.events.push_front(event.into());
    }

    pub fn pop(&mut self) -> Option<Box<dyn AssetEvent>> {
        self.events.pop_front()
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = impl Into<Box<dyn AssetEvent>>>) {
        self.events.extend(events.into_iter().map(Into::into));
    }

    pub(crate) fn observer(
        asset_events: &mut Events<StartAssetEvent>,
        actions: &WorldActions,
        database: &AssetDatabase,
        tasks: &TaskPool,
    ) {
        let mut events = database.events();
        if !events.is_running() {
            events.start();
            events.extend(asset_events.drain().map(|e| e.event));
            let database = database.clone();
            let actions = actions.clone();

            tasks.spawn(move || {
                database.execute(actions);
            });
        } else {
            events.extend(asset_events.drain());
        }
    }
}

pub struct StartAssetEvent {
    pub event: Box<dyn AssetEvent>,
}

impl StartAssetEvent {
    pub fn new(event: impl AssetEvent) -> Self {
        Self {
            event: Box::new(event),
        }
    }
}

impl<A: AssetEvent> From<A> for StartAssetEvent {
    fn from(event: A) -> Self {
        Self {
            event: Box::new(event),
        }
    }
}

impl Into<Box<dyn AssetEvent>> for StartAssetEvent {
    fn into(self) -> Box<dyn AssetEvent> {
        self.event
    }
}

impl Event for StartAssetEvent {}
