use super::{local::LocalAssets, source::AssetPath, AssetIo, AssetIoError};
use crate::asset::{Asset, AssetId};
use futures::{AsyncReadExt, AsyncWriteExt};
use hashbrown::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct AssetCache {
    fs: LocalAssets,
}

impl AssetCache {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            fs: LocalAssets::new(path),
        }
    }

    pub fn artifact_path(&self, id: &AssetId) -> PathBuf {
        self.fs
            .root()
            .join("artifacts")
            .join(id.to_string())
            .join("artifact")
    }

    pub fn temp_artifact_path(&self, id: &AssetId) -> PathBuf {
        self.fs
            .root()
            .join("artifacts")
            .join(id.to_string())
            .join("temp")
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

    pub async fn load_source_map(&self) -> Result<SourceMap, AssetIoError> {
        let path = self.fs.root().join("sources.map");
        if !path.exists() {
            return Ok(SourceMap::new());
        }

        let mut file = self.fs.reader(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        bincode::deserialize(&data).map_err(AssetIoError::from)
    }

    pub async fn save_source_map(&self, map: &SourceMap) -> Result<(), AssetIoError> {
        let path = self.fs.root().join("sources.map");
        let mut file = self.fs.writer(&path).await?;
        let data = bincode::serialize(map).map_err(AssetIoError::from)?;
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
    pub path: AssetPath,
    pub parent: Option<AssetId>,
    pub children: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
    pub processed: Option<ProcessedInfo>,
}

impl ArtifactMeta {
    pub fn new(id: AssetId, path: impl Into<AssetPath>) -> Self {
        Self {
            id,
            path: path.into(),
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessedInfo {
    pub checksum: u32,
    pub dependencies: Vec<AssetDependency>,
}

impl ProcessedInfo {
    pub fn new(checksum: u32) -> Self {
        Self {
            checksum,
            dependencies: Vec::new(),
        }
    }

    pub fn checksum(asset: &[u8], metadata: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(asset);
        hasher.update(metadata);
        hasher.finalize()
    }

    pub fn with_dependencies(mut self, dependencies: Vec<AssetDependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn add_dependency(&mut self, id: AssetId, checksum: u32) {
        self.dependencies.push(AssetDependency { id, checksum });
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AssetDependency {
    pub id: AssetId,
    pub checksum: u32,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceMap {
    map: HashMap<AssetPath, AssetId>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: AssetPath, id: AssetId) {
        self.map.insert(path, id);
    }

    pub fn get(&self, path: &AssetPath) -> Option<AssetId> {
        self.map.get(path).copied()
    }

    pub fn remove(&mut self, path: &AssetPath) -> Option<AssetId> {
        self.map.remove(path)
    }
}
