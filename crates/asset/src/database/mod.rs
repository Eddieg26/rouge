use crate::actions::AssetActions;
use config::AssetConfig;
use ecs::core::resource::Resource;
use std::sync::{Arc, Mutex};

pub mod config;

#[derive(Clone)]
pub struct AssetDatabase {
    config: Arc<AssetConfig>,
    actions: Arc<Mutex<AssetActions>>,
}

impl AssetDatabase {
    pub fn new(config: AssetConfig) -> Self {
        Self {
            config: Arc::new(config),
            actions: Arc::new(Mutex::new(AssetActions::new())),
        }
    }

    pub fn config(&self) -> &Arc<AssetConfig> {
        &self.config
    }

    pub(crate) fn actions(&self) -> Arc<Mutex<AssetActions>> {
        self.actions.clone()
    }

    pub(crate) fn actions_lock(&self) -> std::sync::MutexGuard<'_, AssetActions> {
        self.actions.lock().unwrap()
    }
}

impl Resource for AssetDatabase {}
