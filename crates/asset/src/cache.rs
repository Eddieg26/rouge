use crate::{
    asset::AssetId,
    io::{AssetFuture, AssetIoError, AssetReader, AssetSourceConfig, AssetWriter},
    library::{Artifact, ArtifactHeader, ArtifactMeta},
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
