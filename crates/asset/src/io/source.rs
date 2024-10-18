use super::{AssetFuture, AssetIo, AssetIoError, AssetReader, AssetWriter, PathExt};
use crate::asset::{Asset, AssetMetadata, Settings};
use futures::{AsyncReadExt, AsyncWriteExt, Stream};
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
        write!(f, "{:?}://{}", self.source, self.path.display())?;
        if let Some(name) = &self.name {
            write!(f, "@{}", name)?;
        }
        Ok(())
    }
}

impl AssetPath {
    pub fn new(source: AssetSourceName, path: impl AsRef<Path>) -> Self {
        Self {
            source,
            path: path.as_ref().to_path_buf().into_boxed_path(),
            name: None,
        }
    }

    pub fn with_name(&self, name: impl Into<Box<str>>) -> Self {
        Self {
            name: Some(name.into()),
            ..self.clone()
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

impl From<String> for AssetPath {
    fn from(path: String) -> Self {
        Self::from_str(path)
    }
}

impl From<&str> for AssetPath {
    fn from(path: &str) -> Self {
        Self::from_str(path)
    }
}

impl From<&Path> for AssetPath {
    fn from(path: &Path) -> Self {
        match path.as_os_str().to_str() {
            Some(path) => Self::from_str(path),
            None => Self::new(AssetSourceName::Default, path),
        }
    }
}

impl From<PathBuf> for AssetPath {
    fn from(path: PathBuf) -> Self {
        Self::from(path.as_path())
    }
}

impl From<&AssetPath> for AssetPath {
    fn from(value: &AssetPath) -> Self {
        value.clone()
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
    io: Arc<dyn AssetIo>,
}

impl AssetSource {
    pub fn new<I: AssetIo>(io: I) -> Self {
        Self { io: Arc::new(io) }
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

    pub fn settings_path(path: &Path) -> PathBuf {
        path.append_ext("meta")
    }

    pub async fn read_asset_bytes(&self, path: &Path) -> Result<Vec<u8>, AssetIoError> {
        let mut reader = self.reader(path).await?;
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    pub async fn read_metadata_bytes(&self, path: &Path) -> Result<Vec<u8>, AssetIoError> {
        let mut reader = self.reader(&Self::settings_path(path)).await?;
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    pub async fn load_metadata<A: Asset, S: Settings>(
        &self,
        path: &Path,
    ) -> Result<AssetMetadata<A, S>, AssetIoError> {
        let mut reader = self.reader(&Self::settings_path(path)).await?;
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer).await?;

        ron::from_str::<AssetMetadata<A, S>>(&buffer).map_err(AssetIoError::from)
    }

    pub async fn save_metadata<A: Asset, S: Settings>(
        &self,
        path: &Path,
        settings: &AssetMetadata<A, S>,
    ) -> Result<String, AssetIoError> {
        let mut writer = self.writer(&Self::settings_path(path)).await?;
        let content = ron::to_string(settings).map_err(AssetIoError::from)?;

        writer.write(content.as_bytes()).await?;

        Ok(content)
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

    pub fn add<I: AssetIo>(&mut self, name: AssetSourceName, io: I) {
        self.sources.insert(name, AssetSource::new(io));
    }

    pub fn get(&self, name: &AssetSourceName) -> Option<&AssetSource> {
        self.sources.get(name)
    }

    pub fn contains(&self, name: &AssetSourceName) -> bool {
        self.sources.contains_key(name)
    }
}
