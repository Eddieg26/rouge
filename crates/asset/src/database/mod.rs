use config::AssetConfig;
use ecs::core::resource::Resource;
use events::AssetEvents;
use library::AssetLibrary;
use state::AssetStates;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};

pub mod config;
pub mod events;
pub mod library;
pub mod registry;
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
}

impl Resource for AssetDatabase {}
