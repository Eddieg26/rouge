use crate::{asset::AssetId, io::cache::LoadPath};
use async_std::sync::RwLock;
use ecs::core::resource::Resource;
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

    pub fn is_unloaded_or_failed(self) -> bool {
        matches!(self, LoadState::Unloaded | LoadState::Failed)
    }

    pub fn is_loaded_or_failed(self) -> bool {
        matches!(self, LoadState::Loaded | LoadState::Failed)
    }

    pub fn next_state(&self, incoming: LoadState) -> LoadState {
        match (self, incoming) {
            (LoadState::Loading, _) | (_, LoadState::Loading) => LoadState::Loading,
            (LoadState::Unloaded, _) | (_, LoadState::Unloaded) => LoadState::Loading,
            (LoadState::Failed, _) | (_, LoadState::Failed) => LoadState::Failed,
            (LoadState::Loaded, LoadState::Loaded) => LoadState::Loaded,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetState {
    state: LoadState,
    dependency_state: LoadState,
    loading_dependencies: HashSet<AssetId>,
    failed_dependencies: HashSet<AssetId>,
    dependencies: HashSet<AssetId>,
    dependents: HashSet<AssetId>,
    parent: Option<AssetId>,
    children: HashSet<AssetId>,
}

impl AssetState {
    pub fn new() -> Self {
        Self {
            state: LoadState::Unloaded,
            dependency_state: LoadState::Unloaded,
            loading_dependencies: HashSet::new(),
            failed_dependencies: HashSet::new(),
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            parent: None,
            children: HashSet::new(),
        }
    }

    pub fn with_state(load_state: LoadState) -> Self {
        let mut state = Self::new();
        state.set_state(load_state);
        state
    }

    pub fn state(&self) -> LoadState {
        self.state
    }

    pub fn dependency_state(&self) -> LoadState {
        self.dependency_state
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn dependents(&self) -> &HashSet<AssetId> {
        &self.dependents
    }

    pub fn children(&self) -> &HashSet<AssetId> {
        &self.children
    }

    pub fn set_state(&mut self, state: LoadState) {
        self.state = state;
    }

    pub fn set_parent(&mut self, parent: Option<AssetId>) {
        self.parent = parent;
    }

    pub fn set_dependency_state(&mut self, state: LoadState) {
        self.dependency_state = state;
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

    pub fn update_dependency_state(&mut self) {
        if !self.loading_dependencies.is_empty() {
            self.dependency_state = LoadState::Loading;
        } else if !self.failed_dependencies.is_empty() {
            self.dependency_state = LoadState::Failed;
        } else {
            self.dependency_state = LoadState::Loaded;
        }
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

    pub fn get_load_state(&self, id: AssetId) -> LoadState {
        self.states
            .get(&id)
            .map_or(LoadState::Unloaded, |state| state.state())
    }

    pub fn dependency_state(&self, id: AssetId) -> LoadState {
        self.states
            .get(&id)
            .map_or(LoadState::Unloaded, |state| state.dependency_state())
    }

    pub fn is_fully_loaded(&self, id: &AssetId) -> bool {
        match self.states.get(id) {
            Some(state) => {
                state.state().is_loaded() && state.dependency_state.is_loaded_or_failed()
            }
            None => return false,
        }
    }

    pub fn loading(&mut self, id: AssetId) {
        let mut state = self.states.remove(&id).unwrap_or_else(AssetState::new);
        state.set_state(LoadState::Loading);

        for dep in state.dependents().iter().chain(state.children()) {
            if let Some(state) = self.states.get_mut(dep) {
                state.loading_dependencies.insert(id);
                state.failed_dependencies.remove(&id);
                state.set_state(LoadState::Loading);
            }
        }

        self.states.insert(id, state);
    }

    pub fn loaded(
        &mut self,
        id: AssetId,
        dependencies: Option<Vec<AssetId>>,
        parent: Option<AssetId>,
    ) -> HashSet<AssetId> {
        let mut state = match self.states.remove(&id) {
            Some(state) => state,
            None => AssetState::with_state(LoadState::Loaded),
        };

        if let Some(dependencies) = &dependencies {
            for dep in dependencies.iter().chain(parent.iter()) {
                let load_state = match self.states.get_mut(dep) {
                    Some(dep_state) => {
                        match Some(dep) == parent.as_ref() {
                            true => dep_state.add_child(id),
                            false => dep_state.add_dependent(id),
                        };
                        dep_state.state
                    }
                    None => {
                        let mut dep_state = AssetState::new();
                        match Some(dep) == parent.as_ref() {
                            true => dep_state.add_child(id),
                            false => dep_state.add_dependent(id),
                        };
                        self.states.insert(*dep, dep_state);
                        LoadState::Loading
                    }
                };

                match load_state {
                    LoadState::Loading => state.loading_dependencies.insert(*dep),
                    LoadState::Failed => state.failed_dependencies.insert(*dep),
                    _ => false,
                };
            }
        }

        state.update_dependency_state();
        state.set_state(LoadState::Loaded);
        state.set_parent(parent);

        for dependent in state.dependents().iter().chain(state.children()) {
            let state = match self.states.get_mut(dependent) {
                Some(state) => state,
                None => continue,
            };

            state.loading_dependencies.remove(&id);
            state.failed_dependencies.remove(&id);
            state.update_dependency_state();
        }

        self.states.insert(id, state);

        self.finish_loading(id)
    }

    pub fn unload(&mut self, id: AssetId) -> Option<AssetState> {
        let mut state = self.states.remove(&id)?;
        state.set_state(LoadState::Unloaded);

        for dep in state.dependencies() {
            if let Some(state) = self.states.get_mut(dep) {
                state.remove_dependent(&id);
            }
        }

        for dep in state.dependents() {
            if let Some(state) = self.states.get_mut(dep) {
                state.remove_dependency(&id);
            }
        }

        if let Some(state) = state.parent.and_then(|p| self.states.get_mut(&p)) {
            state.remove_child(&id);
        }

        Some(state)
    }

    pub fn failed(&mut self, id: AssetId) -> HashSet<AssetId> {
        match self.states.get_mut(&id) {
            Some(state) => state.set_state(LoadState::Failed),
            None => return HashSet::new(),
        }

        let state = match self.states.remove(&id) {
            Some(state) => state,
            None => return HashSet::new(),
        };

        for dep in state.dependents() {
            if let Some(state) = self.states.get_mut(dep) {
                state.loading_dependencies.remove(&id);
                state.failed_dependencies.insert(id);
                if !state.loading_dependencies.is_empty() {
                    state.set_state(LoadState::Loading);
                } else {
                    state.set_state(LoadState::Failed);
                }
            }
        }

        self.states.insert(id, state);

        self.finish_loading(id)
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

    pub(crate) fn finish_loading(&self, id: AssetId) -> HashSet<AssetId> {
        let mut finished = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = vec![id];

        while let Some(id) = stack.pop() {
            if visited.contains(&id) {
                continue;
            }

            visited.insert(id);

            let state = match self.states.get(&id) {
                Some(state) => state,
                None => continue,
            };

            if state
                .dependencies()
                .iter()
                .chain(state.parent.iter())
                .all(|dep| finished.contains(dep))
            {
                finished.insert(id);
            }

            stack.extend(state.dependents());
            stack.extend(state.children());
        }

        finished
    }
}

pub type SharedStates = Arc<RwLock<AssetStates>>;

pub struct PreloadAssets {
    pub assets: HashSet<LoadPath>,
}

impl PreloadAssets {
    pub fn new() -> Self {
        Self {
            assets: HashSet::new(),
        }
    }

    pub fn add(&mut self, path: LoadPath) {
        self.assets.insert(path);
    }
}

impl Resource for PreloadAssets {}
