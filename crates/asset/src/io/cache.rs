use super::{local::LocalAssets, source::AssetPath, AssetIo, AssetIoError};
use crate::asset::{Asset, AssetId};
use futures::{AsyncReadExt, AsyncWriteExt};
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

    pub fn prev_artifact_path(&self, id: &AssetId) -> PathBuf {
        self.fs
            .root()
            .join("artifacts")
            .join(id.to_string())
            .join("prev")
    }

    pub async fn load_asset<A: Asset>(&self, id: &AssetId) -> Result<LoadedAsset<A>, AssetIoError> {
        let artifact = self.load_artifact(id).await?;
        let asset = bincode::deserialize::<A>(&artifact.data).map_err(AssetIoError::from)?;
        Ok(LoadedAsset::new(asset, artifact.meta))
    }

    pub async fn save_artifact(
        &self,
        artifact: Artifact,
        checksum: u32,
    ) -> Result<usize, AssetIoError> {
        let path = self.artifact_path(&artifact.meta.id);
        let mut file = self.fs.writer(&path).await?;
        let meta = bincode::serialize(&artifact.meta).map_err(AssetIoError::from)?;
        let header = ArtifactHeader {
            checksum,
            size: meta.len() as u64,
        };
        let mut data = bincode::serialize(&header).map_err(AssetIoError::from)?;
        data.extend(meta);
        data.extend(artifact.data);
        file.write(&data).await.map_err(AssetIoError::from)
    }

    pub async fn load_artifact(&self, id: &AssetId) -> Result<Artifact, AssetIoError> {
        let path = self.artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut header_data = [0; std::mem::size_of::<ArtifactHeader>()];
        file.read_exact(&mut header_data).await?;
        let header =
            bincode::deserialize::<ArtifactHeader>(&header_data).map_err(AssetIoError::from)?;
        let mut data = vec![0; header.size as usize];
        file.read_exact(&mut data).await?;
        let meta = bincode::deserialize(&data).map_err(AssetIoError::from)?;
        let mut data = vec![0; data.len()];
        file.read_exact(&mut data).await?;
        Ok(Artifact { meta, data })
    }

    pub async fn load_artifact_header(&self, id: &AssetId) -> Result<ArtifactHeader, AssetIoError> {
        let path = self.artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut data = [0; std::mem::size_of::<ArtifactHeader>()];
        file.read_exact(&mut data).await?;
        bincode::deserialize(&data).map_err(AssetIoError::from)
    }

    pub async fn load_artifact_meta(&self, id: &AssetId) -> Result<ArtifactMeta, AssetIoError> {
        let path = self.artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        bincode::deserialize(&data).map_err(AssetIoError::from)
    }

    pub async fn save_prev_artifact(&self, id: &AssetId) -> Result<usize, AssetIoError> {
        let artifact = self.load_artifact(id).await?;
        let path = self.prev_artifact_path(id);
        let mut file = self.fs.writer(&path).await?;
        let meta = bincode::serialize(&artifact.meta).map_err(AssetIoError::from)?;
        let header = ArtifactHeader {
            checksum: 0,
            size: meta.len() as u64,
        };
        let mut data = bincode::serialize(&header).map_err(AssetIoError::from)?;
        data.extend(meta);
        data.extend(artifact.data);
        file.write(&data).await.map_err(AssetIoError::from)
    }

    pub async fn load_prev_artifact_meta(
        &self,
        id: &AssetId,
    ) -> Result<ArtifactMeta, AssetIoError> {
        let path = self.prev_artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        bincode::deserialize(&data).map_err(AssetIoError::from)
    }

    pub fn artifact_exists(&self, id: &AssetId) -> bool {
        self.artifact_path(id).exists()
    }

    pub async fn remove_artifact(&self, id: &AssetId) -> Result<(), AssetIoError> {
        let path = self.artifact_path(id);
        self.fs.remove(&path).await
    }

    pub async fn save_temp_artifact(&self, artifact: &Artifact) -> Result<usize, AssetIoError> {
        let path = self.artifact_path(&artifact.meta.id);
        let mut file = self.fs.writer(&path).await?;
        let data = bincode::serialize(&artifact).map_err(AssetIoError::from)?;
        file.write(&data).await.map_err(AssetIoError::from)
    }

    pub async fn load_temp_artifact(&self, id: &AssetId) -> Result<Artifact, AssetIoError> {
        let path = self.artifact_path(id);
        let mut file = self.fs.reader(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        bincode::deserialize(&data).map_err(AssetIoError::from)
    }

    pub async fn remove_temp_artifact(&self, id: &AssetId) -> Result<(), AssetIoError> {
        let path = self.artifact_path(id);
        self.fs.remove(&path).await
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
    checksum: u32,
    /// The size of the artifact metadata in bytes.
    size: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub path: AssetPath,
    pub parent: Option<AssetId>,
    pub children: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
    pub dependents: Vec<AssetId>,
}

impl ArtifactMeta {
    pub fn new(id: AssetId, path: impl Into<AssetPath>) -> Self {
        Self {
            id,
            path: path.into(),
            parent: None,
            children: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
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
