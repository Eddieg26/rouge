use super::{AssetFuture, AssetIo, AssetReader, AssetWriter};
use crate::asset::AssetId;
use futures::Stream;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetPath {
    Id {
        id: AssetId,
    },
    Path {
        source: AssetSourceName,
        path: PathBuf,
    },
}

impl From<AssetId> for AssetPath {
    fn from(id: AssetId) -> Self {
        AssetPath::Id { id }
    }
}

impl From<&AssetId> for AssetPath {
    fn from(value: &AssetId) -> Self {
        AssetPath::Id { id: *value }
    }
}

impl<A: AsRef<Path>> From<A> for AssetPath {
    fn from(value: A) -> Self {
        AssetPath::Path {
            source: AssetSourceName::Default,
            path: value.as_ref().to_path_buf(),
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub enum AssetSourceName {
    #[default]
    Default,
    Name(String),
}

#[derive(Clone)]
pub struct AssetSource {
    path: PathBuf,
    io: Arc<dyn AssetIo>,
}

impl AssetSource {
    pub fn new<I: AssetIo>(path: impl AsRef<Path>, io: I) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            io: Arc::new(io),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reader<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetReader>> {
        self.io.reader(path)
    }

    pub fn read_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<Box<dyn Stream<Item = PathBuf>>> {
        self.io.read_dir(path)
    }

    pub fn is_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<bool> {
        self.io.is_dir(path)
    }

    pub fn writer<'a>(&'a self, path: &'a Path) -> AssetFuture<Box<dyn AssetWriter>> {
        self.io.writer(path)
    }

    pub fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()> {
        self.io.rename(from, to)
    }

    pub fn remove<'a>(&'a self, path: &'a Path) -> AssetFuture<()> {
        self.io.remove(path)
    }

    pub fn remove_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<()> {
        self.io.remove_dir(path)
    }
}

pub struct AssetSources {
    sources: HashMap<AssetSourceName, AssetSource>,
}

impl AssetSources {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn add<I: AssetIo>(&mut self, name: AssetSourceName, path: impl AsRef<Path>, io: I) {
        self.sources.insert(name, AssetSource::new(path, io));
    }

    pub fn get(&self, name: &AssetSourceName) -> Option<&AssetSource> {
        self.sources.get(name)
    }

    pub fn contains(&self, name: &AssetSourceName) -> bool {
        self.sources.contains_key(name)
    }
}
