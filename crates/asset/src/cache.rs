use crate::{
    asset::AssetId,
    io::{AssetFuture, AssetIoError, AssetReader, AssetSourceConfig, AssetWriter},
    library::Artifact,
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

    pub fn save_artifact<'a>(
        &'a self,
        id: AssetId,
        artifact: &'a Artifact,
    ) -> AssetFuture<'a, usize> {
        Box::pin(async move {
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
            let mut data = Vec::new();
            reader.read_to_end(&mut data).await?;
            bincode::deserialize(&data).map_err(|e| match *e {
                bincode::ErrorKind::Io(error) => AssetIoError::Io(Arc::new(error)),
                _ => AssetIoError::Io(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
            })
        })
    }

    pub fn load_artifact_header<'a>(&'a self, id: AssetId) -> AssetFuture<'a, u32> {
        Box::pin(async move {
            let path = self.path.join(id.to_string());
            let mut reader = self.config.reader(&path);
            let mut data = [0; 4];
            reader.read(&mut data).await?;
            bincode::deserialize(&data).map_err(|e| match *e {
                bincode::ErrorKind::Io(error) => AssetIoError::Io(Arc::new(error)),
                _ => AssetIoError::Io(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, e))),
            })
        })
    }
}
