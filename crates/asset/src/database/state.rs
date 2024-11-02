use crate::asset::AssetId;
use async_std::sync::RwLock;
use hashbrown::{hash_map::Entry, HashMap, HashSet};
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

    pub fn is_unloaded_or_failed(self) -> bool {
        matches!(self, LoadState::Unloaded | LoadState::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetState {
    state: LoadState,
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
    parent: Option<AssetId>,
    children: HashSet<AssetId>,
}

impl AssetState {
    pub fn new() -> Self {
        Self {
            state: LoadState::Unloaded,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            parent: None,
            children: HashSet::new(),
        }
    }

    pub fn with_state(state: LoadState) -> Self {
        Self {
            state,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            parent: None,
            children: HashSet::new(),
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

    pub fn parent(&self) -> Option<AssetId> {
        self.parent
    }

    pub fn children(&self) -> &HashSet<AssetId> {
        &self.children
    }

    pub fn set_state(&mut self, state: LoadState) {
        self.state = state;
    }

    pub fn set_parent(&mut self, id: Option<AssetId>) {
        self.parent = id;
    }

    pub fn add_dependent(&mut self, id: AssetId) {
        self.dependents.insert(id);
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_child(&mut self, id: AssetId) {
        self.children.insert(id);
    }

    pub fn remove_dependency(&mut self, id: &AssetId) {
        self.dependencies.remove(id);
    }

    pub fn remove_dependent(&mut self, id: &AssetId) {
        self.dependents.remove(id);
    }

    pub fn remove_child(&mut self, id: &AssetId) {
        self.children.remove(id);
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

    pub fn is_parent_loaded(&self, id: &AssetId) -> bool {
        self.states.get(id).map_or(false, |state| {
            state.parent.map_or(true, |parent| {
                matches!(
                    self.load_state(parent),
                    LoadState::Loaded | LoadState::Failed
                )
            })
        })
    }

    pub fn is_fully_loaded(&self, id: &AssetId) -> bool {
        let state = match self.states.get(id) {
            Some(state) => state,
            None => return false,
        };

        state
            .dependencies
            .iter()
            .all(|dep| matches!(self.load_state(*dep), LoadState::Loaded | LoadState::Failed))
            && state.parent.map_or(true, |parent| {
                matches!(
                    self.load_state(parent),
                    LoadState::Loaded | LoadState::Failed
                )
            })
    }

    pub fn loading(&mut self, id: AssetId) {
        self.states
            .entry(id)
            .or_insert_with(AssetState::new)
            .set_state(LoadState::Loading);
    }

    pub fn loaded(
        &mut self,
        id: AssetId,
        dependencies: Option<Vec<AssetId>>,
        parent: Option<AssetId>,
    ) {
        match self.states.entry(id) {
            Entry::Occupied(entry) => {
                let state = entry.into_mut();
                state.set_state(LoadState::Loaded);
                state.set_parent(parent);
            }
            Entry::Vacant(entry) => {
                let mut state = AssetState::with_state(LoadState::Loaded);
                state.set_parent(parent);
                entry.insert(state);
            }
        }

        if let Some(dependencies) = dependencies {
            for dep in dependencies {
                self.states.get_mut(&id).unwrap().add_dependency(dep);
                match self.states.get_mut(&dep) {
                    Some(state) => state.add_dependency(id),
                    None => {
                        let mut state = AssetState::new();
                        state.add_dependent(id);
                        self.states.insert(dep, state);
                    }
                }
            }
        }

        if let Some(parent) = parent {
            let state = self.states.entry(parent).or_insert_with(AssetState::new);
            state.add_child(id);
        }
    }

    pub fn unload(&mut self, id: AssetId) -> Option<AssetState> {
        let mut state = self.states.remove(&id)?;
        for dep in &state.dependencies {
            if let Some(state) = self.states.get_mut(dep) {
                state.remove_dependent(&id);
            }
        }

        state.set_state(LoadState::Unloaded);

        if let Some(state) = state.parent.and_then(|p| self.states.get_mut(&p)) {
            state.remove_child(&id);
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

    pub fn remove_child(&mut self, id: AssetId, child: AssetId) {
        if let Some(state) = self.states.get_mut(&id) {
            state.remove_child(&child);
        }
    }
}

pub type SharedStates = Arc<RwLock<AssetStates>>;
