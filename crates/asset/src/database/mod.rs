use crate::{config::AssetConfig, library::AssetLibrary};
use ecs::{
    core::resource::Resource,
    event::{Event, Events},
    task::TaskPool,
    world::action::WorldActions,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};

pub mod events;

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    library: Arc<RwLock<AssetLibrary>>,
    events: Arc<Mutex<AssetEvents>>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> Self {
        Self {
            config: Arc::new(config),
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            events: Arc::new(Mutex::new(AssetEvents::new())),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub(crate) fn library_mut(&self) -> std::sync::RwLockWriteGuard<AssetLibrary> {
        self.library.write().unwrap()
    }

    pub(crate) fn events(&self) -> std::sync::MutexGuard<AssetEvents> {
        self.events.lock().unwrap()
    }

    pub(crate) fn execute(&self, actions: WorldActions, events: VecDeque<Box<dyn AssetEvent>>) {
        for mut event in events {
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

    pub(crate) fn observer(
        asset_events: &mut Events<StartAssetEvent>,
        actions: &WorldActions,
        database: &AssetDatabase,
        tasks: &TaskPool,
    ) {
        let mut events = database.events();
        if !events.is_running() {
            events.start();
            let mut events = std::mem::take(&mut events.events);
            asset_events.drain().for_each(|e| events.push_back(e.event));
            let database = database.clone();
            let actions = actions.clone();

            tasks.spawn(move || {
                database.execute(actions, events);
            });
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

impl Event for StartAssetEvent {}
