use crate::{Asset, AssetDependency, AssetId, HashId};
use rouge_ecs::macros::Resource;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Unloaded,
    Unloading,
    Loading,
    Loaded,
    Failed,
}

#[derive(Debug, Clone, Resource)]
pub struct AssetDatabase {
    states: Arc<RwLock<HashMap<AssetId, LoadState>>>,
    dependencies: Arc<RwLock<HashMap<AssetId, HashSet<AssetDependency>>>>,
    dependents: Arc<RwLock<HashMap<AssetId, HashSet<AssetDependency>>>>,
    path_to_id: Arc<RwLock<HashMap<HashId, AssetId>>>,
    assets_imported: Arc<RwLock<bool>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            dependents: Arc::new(RwLock::new(HashMap::new())),
            path_to_id: Arc::new(RwLock::new(HashMap::new())),
            assets_imported: Arc::new(RwLock::new(false)),
        }
    }

    pub fn load_state(&self, id: AssetId) -> LoadState {
        self.states
            .read()
            .unwrap()
            .get(&id)
            .copied()
            .unwrap_or(LoadState::Unloaded)
    }

    pub fn set_load_state(&self, id: AssetId, state: LoadState) {
        self.states.write().unwrap().insert(id, state);
    }

    pub fn set_dependencies<A: Asset>(&self, id: AssetId, dependencies: HashSet<AssetDependency>) {
        for dependency in &dependencies {
            self.dependents
                .write()
                .unwrap()
                .entry(*dependency.id())
                .or_default()
                .insert(AssetDependency::new::<A>(id));
        }

        self.dependencies.write().unwrap().insert(id, dependencies);
    }

    pub fn set_path_id(&self, id: AssetId, path: &Path) {
        let path = HashId::new(&path);
        self.path_to_id.write().unwrap().insert(path, id);
    }

    pub fn id_from_path(&self, path: &Path) -> Option<AssetId> {
        let id = HashId::new(&path);
        self.path_to_id.read().unwrap().get(&id).copied()
    }

    pub fn unload(&self, id: AssetId) {
        self.states.write().unwrap().remove(&id);
        self.dependencies.write().unwrap().remove(&id);
        self.path_to_id.write().unwrap().remove(&HashId::new(&id));
    }

    pub fn dependencies(&self, id: AssetId) -> HashSet<AssetDependency> {
        self.dependencies
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn dependents(&self, id: AssetId) -> HashSet<AssetDependency> {
        self.dependents
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn is_imported(&self) -> bool {
        self.assets_imported.read().unwrap().clone()
    }

    pub fn set_imported(&self) {
        *self.assets_imported.write().unwrap() = true;
    }

    pub fn is_ready(&self, id: AssetId) -> bool {
        let dependencies = self.dependencies(id);
        let state = self.load_state(id);
        state != LoadState::Unloaded
            && state != LoadState::Unloading
            && dependencies.iter().all(|dependency| {
                let state = self.load_state(*dependency.id());
                state != LoadState::Unloaded && state != LoadState::Unloading
            })
    }
}
