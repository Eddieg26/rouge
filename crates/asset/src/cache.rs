use crate::{
    asset::{Asset, AssetId, AssetKind},
    io::{AssetFuture, AssetIoError, AssetReader, AssetSourceConfig, AssetWriter},
};
use std::{path::PathBuf, sync::Arc};

pub struct AssetCache {
    path: PathBuf,
    config: Box<dyn AssetSourceConfig>,
}

impl AssetCache {
    pub fn new<C: AssetSourceConfig>(path: PathBuf, config: C) -> Self {
        Self {
            path,
            config: Box::new(config),
        }
    }

    pub fn reader(&self, path: &PathBuf) -> Box<dyn AssetReader> {
        self.config.reader(path)
    }

    pub fn writer(&self, path: &PathBuf) -> Box<dyn AssetWriter> {
        self.config.writer(path)
    }

    pub fn artifact_path(&self, id: AssetId) -> PathBuf {
        self.path.join(id.to_string())
    }

    pub fn save_artifact<'a>(&'a self, artifact: &'a Artifact) -> AssetFuture<'a, usize> {
        Box::pin(async move {
            let id = artifact.meta.id;
            let path = self.path.join(id.to_string());
            let mut writer = self.config.writer(&path);
            let data = bincode::serialize(artifact).map_err(|e| match *e {
                bincode::ErrorKind::Io(error) => AssetIoError::Io(Arc::new(error)),
                _ => AssetIoError::Io(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
            })?;
            writer.write(&data).await
        })
    }

    pub fn load_artifact<'a>(&'a self, id: AssetId) -> AssetFuture<'a, Artifact> {
        Box::pin(async move {
            let path = self.path.join(id.to_string());
            let mut reader = self.config.reader(&path);
            let data = reader.read_to_end().await?;
            bincode::deserialize(data).map_err(|e| match *e {
                bincode::ErrorKind::Io(error) => AssetIoError::Io(Arc::new(error)),
                _ => AssetIoError::Io(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
            })
        })
    }

    pub fn remove_artifact<'a>(&'a self, id: AssetId) -> AssetFuture<'a, Artifact> {
        Box::pin(async move {
            let artifact = self.load_artifact(id).await?;
            let path = self.path.join(id.to_string());
            let mut writer = self.config.writer(&path);
            writer.remove().await?;
            Ok(artifact)
        })
    }

    pub fn load_artifact_header<'a>(&'a self, id: AssetId) -> AssetFuture<'a, ArtifactHeader> {
        Box::pin(async move {
            let path = self.path.join(id.to_string());
            let mut reader = self.config.reader(&path);
            let data = reader.read(std::mem::size_of::<u32>()).await?;
            bincode::deserialize(&data).map_err(|e| match *e {
                bincode::ErrorKind::Io(error) => AssetIoError::Io(Arc::new(error)),
                _ => AssetIoError::Io(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
            })
        })
    }

    pub fn load_artifact_meta<'a>(&'a self, id: &'a AssetId) -> AssetFuture<'a, ArtifactMeta> {
        Box::pin(async move {
            let path = self.path.join(id.to_string());
            let mut reader = self.config.reader(&path);
            let header = reader.read(std::mem::size_of::<u32>()).await?;
            let header = bincode::deserialize::<ArtifactHeader>(&header).map_err(|e| {
                AssetIoError::from(std::io::Error::new(std::io::ErrorKind::Other, e))
            })?;

            let bytes = reader.read(header.meta_size as usize).await?;
            bincode::deserialize::<ArtifactMeta>(&bytes)
                .map_err(|e| AssetIoError::from(std::io::Error::new(std::io::ErrorKind::Other, e)))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub path: PathBuf,
    pub sub_assets: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
    pub kind: AssetKind,
}

impl ArtifactMeta {
    pub fn main(
        id: AssetId,
        path: PathBuf,
        dependencies: Vec<AssetId>,
        sub_assets: Vec<AssetId>,
    ) -> Self {
        Self {
            id,
            path,
            sub_assets,
            dependencies,
            kind: AssetKind::Main,
        }
    }

    pub fn sub(sub: AssetId, main: AssetId, path: PathBuf) -> Self {
        Self {
            id: sub,
            path,
            sub_assets: vec![],
            dependencies: vec![main],
            kind: AssetKind::Sub,
        }
    }

    pub fn add_sub_asset(&mut self, sub_asset: AssetId) {
        self.sub_assets.push(sub_asset);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactHeader {
    pub meta_size: u32,
    pub checksum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub header: ArtifactHeader,
    pub meta: ArtifactMeta,
    asset: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: Vec<u8>, meta: ArtifactMeta, checksum: u32) -> Self {
        let meta_size = bincode::serialized_size(&meta).unwrap() as u32;
        Self {
            header: ArtifactHeader {
                meta_size,
                checksum,
            },
            meta,
            asset,
        }
    }

    pub fn from_asset<A: Asset>(asset: A, meta: ArtifactMeta, checksum: u32) -> Self {
        let asset = bincode::serialize(&asset).unwrap();
        Self::new(asset, meta, checksum)
    }

    pub fn header(&self) -> &ArtifactHeader {
        &self.header
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn asset(&self) -> &[u8] {
        &self.asset
    }
}
