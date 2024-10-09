use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    string::FromUtf8Error,
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

impl From<toml::ser::Error> for AssetIoError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Io(Arc::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            err,
        )))
    }
}

impl From<toml::de::Error> for AssetIoError {
    fn from(err: toml::de::Error) -> Self {
        Self::Io(Arc::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            err,
        )))
    }
}

impl From<FromUtf8Error> for AssetIoError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Io(Arc::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err,
        )))
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

pub type AssetFuture<'a, T, E = AssetIoError> = Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>;

pub trait AssetReader: Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> AssetFuture<'a, usize>;
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize>;
    fn read_directory<'a>(&'a mut self) -> AssetFuture<'a, Vec<PathBuf>>;
    fn is_directory<'a>(&'a mut self) -> AssetFuture<'a, bool>;
}

pub trait AssetWriter: Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn write<'a>(&'a mut self, data: &'a [u8]) -> AssetFuture<'a, usize>;
    fn create_directory<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn remove<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn remove_directory<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn rename<'a>(&'a mut self, to: &'a Path) -> AssetFuture<'a, ()>;
}

pub trait AssetWatcher: Send + Sync + 'static {
    fn watch(&self, path: &Path) -> Result<(), AssetIoError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SourceId {
    Default,
    Root(Box<str>),
}

impl From<String> for SourceId {
    fn from(s: String) -> Self {
        SourceId::Root(s.into_boxed_str())
    }
}

impl From<&str> for SourceId {
    fn from(s: &str) -> Self {
        SourceId::Root(s.to_string().into_boxed_str())
    }
}

pub trait AssetSourceConfig: downcast_rs::Downcast + Send + Sync + 'static {
    fn reader(&self, path: &Path) -> Box<dyn AssetReader>;
    fn writer(&self, path: &Path) -> Box<dyn AssetWriter>;
    fn watcher(&self, path: &Path) -> Box<dyn AssetWatcher>;
}
downcast_rs::impl_downcast!(AssetSourceConfig);

pub struct AssetSource {
    config: Box<dyn AssetSourceConfig>,
}

impl AssetSource {
    pub fn new<C: AssetSourceConfig>(config: C) -> Self {
        Self {
            config: Box::new(config),
        }
    }

    pub fn config(&self) -> &dyn AssetSourceConfig {
        &*self.config
    }

    pub fn config_as<C: AssetSourceConfig>(&self) -> Option<&C> {
        self.config.downcast_ref()
    }

    pub fn reader(&self, path: &Path) -> Box<dyn AssetReader> {
        self.config.reader(path)
    }

    pub fn writer(&self, path: &Path) -> Box<dyn AssetWriter> {
        self.config.writer(path)
    }

    pub fn watcher(&self, path: &Path) -> Box<dyn AssetWatcher> {
        self.config.watcher(path)
    }
}

pub trait PathExt {
    fn ext(&self) -> Option<&str>;
    fn append_ext(&self, ext: &str) -> PathBuf;
    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf;
    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path;
}

impl<T: AsRef<Path>> PathExt for T {
    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }

    fn append_ext(&self, ext: &str) -> PathBuf {
        let path = self.as_ref().to_path_buf();
        format!("{}.{}", path.display(), ext).into()
    }

    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf {
        match self.as_ref().starts_with(prefix.as_ref()) {
            true => self.as_ref().to_path_buf(),
            false => prefix.as_ref().join(self.as_ref()),
        }
    }

    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path {
        let path = self.as_ref();
        let prefix = prefix.as_ref();

        if path.starts_with(prefix) {
            path.strip_prefix(prefix).unwrap()
        } else {
            path
        }
    }
}

pub struct AssetSources {
    sources: HashMap<SourceId, Arc<AssetSource>>,
}
