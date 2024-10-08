use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

pub mod embed;
pub mod local;

#[derive(Debug, Clone)]
pub enum AssetIoError {
    Io(Arc<std::io::Error>),
    Http(u16),
    NotFound(PathBuf),
}

impl From<std::io::Error> for AssetIoError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(Arc::new(err))
    }
}

impl std::fmt::Display for AssetIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err),
            Self::Http(status) => write!(f, "HTTP status code: {}", status),
            Self::NotFound(path) => write!(f, "Asset not found: {:?}", path),
        }
    }
}

impl std::error::Error for AssetIoError {}

pub type AssetFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AssetIoError>> + 'a>>;

pub trait AssetReader: Send + Sync + 'static {
    fn read<'a>(&'a mut self, path: &'a Path, buf: &'a mut [u8]) -> AssetFuture<'a, usize>;
    fn read_to_end<'a>(&mut self, path: &'a Path, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize>;
    fn read_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, Vec<PathBuf>>;
    fn is_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, bool>;
}

pub trait AssetWriter: Send + Sync + 'static {
    fn write<'a>(&'a mut self, path: &'a Path, data: &'a [u8]) -> AssetFuture<'a, usize>;
    fn create_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn remove<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn remove_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn rename<'a>(&'a mut self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()>;
}

pub trait AssetWatcher: Send + Sync + 'static {
    fn watch(&self, path: &Path) -> Result<(), AssetIoError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetSourceRoot(Box<str>);
impl From<String> for AssetSourceRoot {
    fn from(s: String) -> Self {
        Self(s.into_boxed_str())
    }
}

impl From<&str> for AssetSourceRoot {
    fn from(s: &str) -> Self {
        Self(s.to_string().into_boxed_str())
    }
}

impl AsRef<str> for AssetSourceRoot {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub trait AssetSourceConfig: Send + Sync + 'static {
    fn reader(&self) -> Box<dyn AssetReader>;
    fn writer(&self) -> Box<dyn AssetWriter>;
    fn watcher(&self) -> Box<dyn AssetWatcher>;
}

pub struct AssetSource {
    root: AssetSourceRoot,
    config: Box<dyn AssetSourceConfig>,
}

impl AssetSource {
    pub fn new<C: AssetSourceConfig>(root: AssetSourceRoot, config: C) -> Self {
        Self {
            root,
            config: Box::new(config),
        }
    }

    pub fn root(&self) -> &str {
        self.root.as_ref()
    }

    pub fn reader(&self) -> Box<dyn AssetReader> {
        self.config.reader()
    }

    pub fn writer(&self) -> Box<dyn AssetWriter> {
        self.config.writer()
    }

    pub fn watcher(&self) -> Box<dyn AssetWatcher> {
        self.config.watcher()
    }
}

pub struct AssetSources {
    sources: HashMap<AssetSourceRoot, Arc<AssetSource>>,
}
