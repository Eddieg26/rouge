use super::{local::LocalFs, source::AssetPath, vfs::VirtualFs, AssetIoError, ErasedFileSystem};
use crate::asset::{Asset, AssetId, AssetType, ErasedAsset, MetaMode};
use async_std::sync::RwLock;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use hashbrown::HashMap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct AssetCache {
    fs: Box<dyn ErasedFileSystem>,
    meta_mode: MetaMode,
}

impl AssetCache {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            fs: Box::new(LocalFs::new(path)),
            meta_mode: MetaMode::Text,
        }
    }

    pub fn test() -> Self {
        Self {
            fs: Box::new(VirtualFs::new(".cache")),
            meta_mode: MetaMode::Binary,
        }
    }

    pub async fn init(&self) -> Result<AssetLibrary, AssetIoError> {
        let root = self.fs.root();
        if !root.exists() {
            self.fs.create_dir_all(&root).await?;
        }
        let artifacts = root.join("artifacts");
        if !artifacts.exists() {
            self.fs.create_dir_all(&artifacts).await?;
        }

        let library = match self.load_library().await {
            Ok(library) => library,
            Err(_) => {
                let library = AssetLibrary::new();
                self.save_library(&library).await?;
                library
            }
        };

        Ok(library)
    }

    pub fn root(&self) -> &Path {
        self.fs.root()
    }

    pub fn artifact_path(&self, id: &AssetId) -> PathBuf {
        self.fs.root().join("artifacts").join(id.to_string())
    }

    pub fn unprocessed_artifact_path(&self, id: &AssetId) -> PathBuf {
        self.fs.root().join("temp").join(id.to_string())
    }

    pub async fn create_unprocessed_dir(&self) -> Result<(), AssetIoError> {
        self.fs.create_dir_all(&self.fs.root().join("temp")).await
    }

    pub async fn remove_unprocessed_dir(&self) -> Result<(), AssetIoError> {
        let path = self.fs.root().join("temp");
        self.fs.remove_dir(&path).await
    }

    pub async fn load_asset<A: Asset>(&self, id: &AssetId) -> Result<LoadedAsset<A>, AssetIoError> {
        let path = self.artifact_path(id);
        let artifact = self.load_artifact(&path).await?;
        let asset = bincode::deserialize::<A>(&artifact.data).map_err(AssetIoError::from)?;
        Ok(LoadedAsset::new(asset, artifact.meta))
    }

    pub async fn load_artifact(&self, path: &Path) -> Result<Artifact, AssetIoError> {
        let mut file = self.fs.reader(&path).await?;
        let mut header = [0; std::mem::size_of::<ArtifactHeader>()];
        file.read_exact(&mut header).await?;
        let header = bincode::deserialize::<ArtifactHeader>(&header).map_err(AssetIoError::from)?;
        let mut meta = vec![0; header.size as usize];
        file.read_exact(&mut meta).await?;
        let meta = bincode::deserialize::<ArtifactMeta>(&meta).map_err(AssetIoError::from)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        Ok(Artifact { meta, data })
    }

    pub async fn save_artifact(
        &self,
        path: &Path,
        artifact: &Artifact,
    ) -> Result<(), AssetIoError> {
        let mut file = self.fs.writer(&path).await?;
        let meta = bincode::serialize(&artifact.meta).map_err(AssetIoError::from)?;
        let mut data = bincode::serialize(&ArtifactHeader::new(meta.len() as u64))
            .map_err(AssetIoError::from)?;
        data.extend_from_slice(&meta);
        data.extend_from_slice(&artifact.data);
        file.write_all(&data).await.map_err(AssetIoError::from)
    }

    pub async fn load_artifact_meta(&self, id: &AssetId) -> Result<ArtifactMeta, AssetIoError> {
        let path = self.artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut header = [0; std::mem::size_of::<ArtifactHeader>()];
        file.read_exact(&mut header).await?;
        let header = bincode::deserialize::<ArtifactHeader>(&header).map_err(AssetIoError::from)?;
        let mut meta = vec![0; header.size as usize];
        file.read_exact(&mut meta).await?;
        bincode::deserialize(&meta).map_err(AssetIoError::from)
    }

    pub fn artifact_exists(&self, id: &AssetId) -> bool {
        self.artifact_path(id).exists()
    }

    pub fn remove_artifact<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl futures::Future<Output = Result<(), AssetIoError>> + 'a {
        self.fs.remove(path)
    }

    pub async fn load_library(&self) -> Result<AssetLibrary, AssetIoError> {
        let path = self.fs.root().join("assets.lib");
        if !path.exists() {
            return Ok(AssetLibrary::new());
        }

        let mut file = self.fs.reader(&path).await?;
        let mut data = String::new();
        file.read_to_string(&mut data).await?;
        match self.meta_mode {
            MetaMode::Text => ron::from_str(&data).map_err(AssetIoError::from),
            MetaMode::Binary => bincode::deserialize(data.as_bytes()).map_err(AssetIoError::from),
        }
    }

    pub async fn save_library(&self, map: &AssetLibrary) -> Result<(), AssetIoError> {
        let path = self.fs.root().join("assets.lib");
        let mut file = self.fs.writer(&path).await?;
        let data = match self.meta_mode {
            MetaMode::Text => ron::to_string(map)
                .map_err(AssetIoError::from)?
                .into_bytes(),
            MetaMode::Binary => bincode::serialize(map).map_err(AssetIoError::from)?,
        };
        file.write_all(&data).await.map_err(AssetIoError::from)
    }
}

pub struct LoadedAsset<A: Asset> {
    asset: A,
    meta: ArtifactMeta,
}

impl<A: Asset> LoadedAsset<A> {
    pub fn new(asset: A, meta: ArtifactMeta) -> Self {
        Self { asset, meta }
    }

    pub fn asset(&self) -> &A {
        &self.asset
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }
}

pub struct ErasedLoadedAsset {
    pub asset: ErasedAsset,
    pub meta: ArtifactMeta,
}

impl ErasedLoadedAsset {
    pub fn new<A: Asset>(asset: A, meta: ArtifactMeta) -> ErasedLoadedAsset {
        Self {
            asset: ErasedAsset::new(asset),
            meta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ArtifactHeader {
    /// The size of the artifact metadata in bytes.
    size: u64,
}

impl ArtifactHeader {
    pub fn new(size: u64) -> Self {
        Self { size }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub checksum: u32,
    pub path: AssetPath,
    pub parent: Option<AssetId>,
    pub children: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
    pub processed: Option<ProcessedInfo>,
}

impl ArtifactMeta {
    pub fn new(id: AssetId, path: AssetPath, checksum: u32) -> Self {
        Self {
            id,
            checksum,
            path,
            parent: None,
            children: Vec::new(),
            dependencies: Vec::new(),
            processed: None,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_parent(mut self, parent: AssetId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_children(mut self, children: Vec<AssetId>) -> Self {
        self.children = children;
        self
    }

    pub fn with_processed(mut self, processed: ProcessedInfo) -> Self {
        self.processed = Some(processed);
        self
    }

    pub fn calculate_checksum(asset: &[u8], metadata: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(metadata);
        hasher.finalize()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub meta: ArtifactMeta,
    pub data: Vec<u8>,
}

impl Artifact {
    pub fn from_asset<A: Asset>(asset: &A, meta: ArtifactMeta) -> Result<Self, bincode::Error> {
        Ok(Self {
            meta,
            data: bincode::serialize(asset)?,
        })
    }

    pub fn id(&self) -> AssetId {
        self.meta.id
    }

    pub fn ty(&self) -> AssetType {
        self.id().ty()
    }

    pub fn path(&self) -> &AssetPath {
        &self.meta.path
    }

    pub fn deserialize<A: Asset>(&self) -> Result<A, bincode::Error> {
        bincode::deserialize(&self.data)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessedInfo {
    pub dependencies: Vec<AssetInfo>,
}

impl ProcessedInfo {
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetInfo>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn add_dependency(&mut self, id: AssetId, checksum: u32) {
        self.dependencies.push(AssetInfo { id, checksum });
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AssetInfo {
    pub id: AssetId,
    pub checksum: u32,
}

impl AssetInfo {
    pub fn new(id: AssetId, checksum: u32) -> Self {
        Self { id, checksum }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AssetLibrary {
    assets: HashMap<AssetPath, AssetInfo>,
    paths: HashMap<AssetId, AssetPath>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn add(&mut self, path: AssetPath, info: AssetInfo) {
        self.paths.insert(info.id, path.clone());
        self.assets.insert(path, info);
    }

    pub fn remove(&mut self, path: &AssetPath) -> Option<AssetInfo> {
        let info = self.assets.remove(path)?;
        self.paths.remove(&info.id);
        Some(info)
    }

    pub fn get(&self, path: &AssetPath) -> Option<AssetInfo> {
        self.assets.get(path).copied()
    }

    pub fn get_id(&self, path: &AssetPath) -> Option<AssetId> {
        self.assets.get(path).map(|info| info.id)
    }

    pub fn get_path(&self, id: &AssetId) -> Option<&AssetPath> {
        self.paths.get(id)
    }

    pub fn contains(&self, path: &AssetPath) -> bool {
        self.assets.contains_key(path)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetPath, &AssetInfo)> {
        self.assets.iter()
    }

    pub fn paths(&self) -> impl Iterator<Item = &AssetPath> {
        self.assets.keys()
    }

    pub fn ids(&self) -> impl Iterator<Item = &AssetId> {
        self.paths.keys()
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

pub type SharedLibrary = Arc<RwLock<AssetLibrary>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetLoadPath {
    Id(AssetId),
    Path(AssetPath),
}

impl From<AssetId> for AssetLoadPath {
    fn from(id: AssetId) -> Self {
        AssetLoadPath::Id(id)
    }
}

impl<I: Into<AssetPath>> From<I> for AssetLoadPath {
    fn from(path: I) -> Self {
        AssetLoadPath::Path(path.into())
    }
}
