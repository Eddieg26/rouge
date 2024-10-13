use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    path::{Path, PathBuf},
    pin::Pin,
    string::FromUtf8Error,
    sync::Arc,
};

use crate::asset::{AssetSettings, Settings};

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
    fn read<'a>(&'a mut self, amount: usize) -> AssetFuture<'a, &'a [u8]>;
    fn read_to_end<'a>(&'a mut self) -> AssetFuture<'a, &'a [u8]>;
    fn read_directory<'a>(&'a mut self) -> AssetFuture<'a, Vec<PathBuf>>;
    fn is_directory<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool>;
    fn exists<'a>(&'a self) -> AssetFuture<'a, bool>;
    fn flush<'a>(&'a mut self) -> AssetFuture<'a, Vec<u8>>;
}

pub trait AssetWriter: Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn write<'a>(&'a mut self, data: &'a [u8]) -> AssetFuture<'a, usize>;
    fn create_directory<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn remove<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn remove_directory<'a>(&'a mut self) -> AssetFuture<'a, ()>;
    fn rename<'a>(&'a mut self, to: &'a Path) -> AssetFuture<'a, ()>;
    fn flush<'a>(&'a mut self) -> AssetFuture<'a, ()>;
}

pub trait AssetWatcher: Send + Sync + 'static {
    fn watch(&self, path: &Path) -> Result<(), AssetIoError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SourceId {
    Default,
    Id(u32),
}

impl From<String> for SourceId {
    fn from(s: String) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        s.hash(&mut hasher);
        SourceId::Id(hasher.finalize())
    }
}

impl From<&str> for SourceId {
    fn from(s: &str) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        s.hash(&mut hasher);
        SourceId::Id(hasher.finalize())
    }
}

pub trait AssetSourceConfig: downcast_rs::Downcast + Send + Sync + 'static {
    fn reader(&self, path: &Path) -> Box<dyn AssetReader>;
    fn writer(&self, path: &Path) -> Box<dyn AssetWriter>;
    fn watcher(&self, path: &Path) -> Box<dyn AssetWatcher>;
}
downcast_rs::impl_downcast!(AssetSourceConfig);

pub struct AssetSource {
    id: SourceId,
    config: Box<dyn AssetSourceConfig>,
}

impl AssetSource {
    pub fn new<C: AssetSourceConfig>(id: SourceId, config: C) -> Self {
        Self {
            id,
            config: Box::new(config),
        }
    }

    pub fn id(&self) -> &SourceId {
        &self.id
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

    pub fn settings_path(&self, path: &Path) -> PathBuf {
        path.append_ext("meta")
    }

    pub fn save_settings<'a, S: Settings>(
        &'a self,
        path: &'a Path,
        settings: &'a AssetSettings<S>,
    ) -> AssetFuture<'a, usize> {
        Box::pin(async move {
            let mut writer = self.writer(&self.settings_path(&path));
            let value = toml::to_string(settings).map_err(|e| AssetIoError::from(e))?;
            writer.write(value.as_bytes()).await
        })
    }

    pub fn load_settings<'a, S: Settings>(
        &'a self,
        path: &'a Path,
    ) -> AssetFuture<'a, AssetSettings<S>> {
        Box::pin(async move {
            let mut reader = self.reader(&self.settings_path(&path));
            reader.read_to_end().await?;
            let data = reader.flush().await?;
            let value = String::from_utf8(data).map_err(|e| AssetIoError::from(e))?;
            toml::from_str(&value).map_err(|e| AssetIoError::from(e))
        })
    }

    pub fn checksum(asset: &[u8], settings: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        settings.hash(&mut hasher);
        asset.hash(&mut hasher);
        hasher.finalize()
    }
}

pub struct AssetSources {
    sources: HashMap<SourceId, AssetSource>,
}

impl AssetSources {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn add<C: AssetSourceConfig>(&mut self, id: SourceId, config: C) {
        self.sources
            .insert(id.clone(), AssetSource::new(id, config));
    }

    pub fn remove(&mut self, id: &SourceId) {
        self.sources.remove(id);
    }

    pub fn get(&self, id: &SourceId) -> Option<&AssetSource> {
        self.sources.get(id)
    }

    pub fn contains(&self, id: &SourceId) -> bool {
        self.sources.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
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
