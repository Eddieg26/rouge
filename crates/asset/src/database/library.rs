use crate::{asset::AssetId, io::SourceId};
use hashbrown::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SourceLibrary {
    map: HashMap<PathBuf, AssetId>,
}

impl SourceLibrary {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: PathBuf, id: AssetId) {
        self.map.insert(path, id);
    }

    pub fn remove(&mut self, path: &Path) -> Option<AssetId> {
        self.map.remove(path)
    }

    pub fn get(&self, path: &Path) -> Option<AssetId> {
        self.map.get(path).copied()
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.map.contains_key(path)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AssetLibrary {
    sources: HashMap<SourceId, SourceLibrary>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        let mut sources = HashMap::new();
        sources.insert(SourceId::Default, SourceLibrary::new());
        Self { sources }
    }

    pub fn add_sources(&mut self, id: SourceId) {
        self.sources.insert(id, SourceLibrary::new());
    }

    pub fn remove_sources(&mut self, id: &SourceId) {
        self.sources.remove(id);
    }

    pub fn get_sources(&self, id: &SourceId) -> Option<&SourceLibrary> {
        self.sources.get(id)
    }

    pub fn get_sources_mut(&mut self, id: &SourceId) -> &mut SourceLibrary {
        self.sources.entry(*id).or_insert_with(SourceLibrary::new)
    }

    pub fn contains_sources(&self, id: &SourceId) -> bool {
        self.sources.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}
