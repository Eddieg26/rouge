use super::{AssetIoError, AssetReader, AssetWriter, FileSystem, PathExt, PathStream};
use async_std::fs::File;
use futures_lite::{AsyncReadExt, StreamExt};
use std::path::Path;

impl AssetReader for File {
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> super::AssetFuture<'a, usize> {
        Box::pin(async move {
            AsyncReadExt::read_to_end(self, buf)
                .await
                .map_err(|e| AssetIoError::from(e))
        })
    }
}

impl AssetWriter for File {}

#[derive(Clone)]
pub struct LocalFs {
    root: Box<Path>,
}

impl LocalFs {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf().into_boxed_path(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl FileSystem for LocalFs {
    type Reader = File;
    type Writer = File;

    fn root(&self) -> &Path {
        &self.root
    }

    async fn reader(&self, path: &std::path::Path) -> Result<Self::Reader, AssetIoError> {
        let path = path.with_prefix(&self.root);
        let file = File::open(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;

        Ok(file)
    }

    async fn read_dir(&self, path: &std::path::Path) -> Result<Box<dyn PathStream>, AssetIoError> {
        let path = path.with_prefix(&self.root);
        let root = self.root.to_path_buf();
        let stream = async_std::fs::read_dir(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?
            .filter_map(move |entry| match entry {
                Ok(e) => Some(e.path().without_prefix(&root).into()),
                Err(_) => None,
            });

        Ok(Box::new(stream) as Box<dyn PathStream>)
    }

    async fn is_dir(&self, path: &std::path::Path) -> Result<bool, AssetIoError> {
        let path = path.with_prefix(&self.root);
        let metadata = async_std::fs::metadata(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;
        Ok(metadata.is_dir())
    }

    async fn writer(&self, path: &std::path::Path) -> Result<File, AssetIoError> {
        let path = path.with_prefix(&self.root);
        let file = File::create(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;
        Ok(file)
    }

    async fn create_dir(&self, path: &Path) -> Result<(), AssetIoError> {
        let path = path.with_prefix(&self.root);
        async_std::fs::create_dir(path.as_ref())
            .await
            .map_err(AssetIoError::from)
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), AssetIoError> {
        let path = path.with_prefix(&self.root);
        async_std::fs::create_dir_all(path.as_ref())
            .await
            .map_err(AssetIoError::from)
    }

    async fn rename(
        &self,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> Result<(), AssetIoError> {
        let from = from.with_prefix(&self.root);
        let to = to.with_prefix(&self.root);
        async_std::fs::rename(from.as_ref(), to.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn remove(&self, path: &std::path::Path) -> Result<(), AssetIoError> {
        let path = path.with_prefix(&self.root);
        async_std::fs::remove_file(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn remove_dir(&self, path: &std::path::Path) -> Result<(), AssetIoError> {
        let path = path.with_prefix(&self.root);
        async_std::fs::remove_dir_all(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn exists(&self, path: &Path) -> Result<bool, AssetIoError> {
        let path = path.with_prefix(&self.root);
        Ok(path.exists())
    }
}
