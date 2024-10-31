use crate::asset::AssetId;
use async_std::sync::RwLock;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Unloaded,
    Loading,
    Loaded,
    Failed,
}

impl LoadState {
    pub fn is_loading(self) -> bool {
        matches!(self, LoadState::Loading)
    }

    pub fn is_loaded(self) -> bool {
        matches!(self, LoadState::Loaded)
    }

    pub fn is_failed(self) -> bool {
        matches!(self, LoadState::Failed)
    }

    pub fn is_unloaded(self) -> bool {
        matches!(self, LoadState::Unloaded)
    }
}

pub struct AssetState {
    state: LoadState,
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
}

impl AssetState {
    pub fn new() -> Self {
        Self {
            state: LoadState::Unloaded,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn state(&self) -> LoadState {
        self.state
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn dependents(&self) -> &HashSet<AssetId> {
        &self.dependents
    }

    pub fn set_state(&mut self, state: LoadState) {
        self.state = state;
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) {
        self.dependencies.remove(id);
    }

    pub fn remove_dependent(&mut self, id: &AssetId) {
        self.dependents.remove(id);
    }
}

#[derive(Default)]
pub struct AssetStates {
    states: HashMap<AssetId, AssetState>,
}

impl AssetStates {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&AssetState> {
        self.states.get(id)
    }

    pub fn load_state(&self, id: AssetId) -> LoadState {
        self.states
            .get(&id)
            .map_or(LoadState::Unloaded, |state| state.state())
    }

    pub fn is_deps_loaded(&self, id: &AssetId) -> bool {
        self.states.get(id).map_or(false, |state| {
            state
                .dependencies
                .iter()
                .all(|dep| matches!(self.load_state(*dep), LoadState::Loaded | LoadState::Failed))
        })
    }

    pub fn loading(&mut self, id: AssetId) {
        self.states
            .entry(id)
            .or_insert_with(AssetState::new)
            .set_state(LoadState::Loading);
    }

    pub fn loaded(&mut self, id: AssetId, dependencies: Option<Vec<AssetId>>) {
        self.states
            .entry(id)
            .or_insert_with(AssetState::new)
            .set_state(LoadState::Loaded);

        if let Some(dependencies) = dependencies {
            for dependency in dependencies {
                self.states
                    .entry(dependency)
                    .or_insert_with(AssetState::new)
                    .add_dependent(id);
            }
        }
    }

    pub fn unload(&mut self, id: AssetId) -> Option<AssetState> {
        let state = self.states.remove(&id)?;
        for dep in &state.dependencies {
            if let Some(state) = self.states.get_mut(dep) {
                state.remove_dependent(&id);
            }
        }

        Some(state)
    }

    pub fn failed(&mut self, id: AssetId) -> &AssetState {
        let state = self.states.entry(id).or_insert_with(AssetState::new);
        state.set_state(LoadState::Failed);
        state
    }

    pub fn remove_dependent(&mut self, id: AssetId, dependent: AssetId) {
        if let Some(state) = self.states.get_mut(&id) {
            state.remove_dependent(&dependent);
        }
    }
}

pub type SharedStates = Arc<RwLock<AssetStates>>;
