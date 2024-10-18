use super::{AssetFuture, AssetIo, AssetReader, AssetWriter};
use futures::Stream;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetPath {
    source: AssetSourceName,
    path: Box<Path>,
    name: Option<Box<str>>,
}

impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}", self.source, self.path.display())?;
        if let Some(name) = &self.name {
            write!(f, "@{}", name)?;
        }
        Ok(())
    }
}

pub enum AssetPathParseError {}

impl AssetPath {
    pub fn new(source: AssetSourceName, path: impl AsRef<Path>) -> Self {
        Self {
            source,
            path: path.as_ref().to_path_buf().into_boxed_path(),
            name: None,
        }
    }

    pub fn source(&self) -> &AssetSourceName {
        &self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// remote://assets/texture.png@main
    pub fn from_str<A: AsRef<str>>(path: A) -> Self {
        let path = path.as_ref();
        let (source, src_index) = match path.find("://") {
            Some(position) => {
                let source = &path[..position];
                (AssetSourceName::Name(source.to_string()), position + 3)
            }
            None => (AssetSourceName::Default, 0),
        };

        let (name, name_index) = match path[src_index..].find('@') {
            Some(position) => {
                let name = &path[src_index + position + 1..];
                (Some(name.to_string()), src_index + position)
            }
            None => (None, path.len()),
        };

        let path = &path[src_index..name_index];
        let path = Path::new(path);

        Self {
            source,
            path: path.to_path_buf().into_boxed_path(),
            name: name.map(|name| name.into_boxed_str()),
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub enum AssetSourceName {
    #[default]
    Default,
    Name(String),
}

impl std::fmt::Display for AssetSourceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetSourceName::Default => write!(f, "default"),
            AssetSourceName::Name(name) => write!(f, "{}", name),
        }
    }
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
