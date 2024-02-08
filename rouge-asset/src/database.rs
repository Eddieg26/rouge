use crate::{AssetId, HashId};
use rouge_ecs::{macros::Resource, resource::Resource};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    None,
    Unloading,
    Loading,
    Loaded,
}

#[derive(Debug, Clone, Resource)]
pub struct AssetDatabase {
    states: Arc<RwLock<HashMap<AssetId, LoadState>>>,
    dependencies: Arc<RwLock<HashMap<AssetId, HashSet<AssetId>>>>,
    path_to_id: Arc<RwLock<HashMap<HashId, AssetId>>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            path_to_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load_state(&self, id: AssetId) -> LoadState {
        self.states
            .read()
            .unwrap()
            .get(&id)
            .copied()
            .unwrap_or(LoadState::None)
    }

    pub fn set_load_state(&self, id: AssetId, state: LoadState) {
        self.states.write().unwrap().insert(id, state);
    }

    pub fn set_dependencies(&mut self, id: AssetId, dependencies: HashSet<AssetId>) {
        self.dependencies.write().unwrap().insert(id, dependencies);
    }

    pub fn set_path_id(&mut self, id: AssetId, path: &Path) {
        let path = HashId::new(&path);
        self.path_to_id.write().unwrap().insert(path, id);
    }

    pub fn id_from_path(&self, path: &Path) -> Option<AssetId> {
        let id = HashId::new(&path);
        self.path_to_id.read().unwrap().get(&id).copied()
    }

    pub fn unload(&mut self, id: AssetId) {
        self.states.write().unwrap().remove(&id);
        self.path_to_id.write().unwrap().remove(&HashId::new(&id));
    }

    pub fn dependencies(&self, id: AssetId) -> HashSet<AssetId> {
        self.dependencies
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn is_ready(&self, id: AssetId) -> bool {
        let dependencies = self.dependencies(id);
        dependencies
            .iter()
            .all(|id| self.load_state(*id) == LoadState::Loaded)
    }
}
