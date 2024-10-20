use super::{AssetIo, AssetIoError, AssetReader, AssetWriter, PathStream};
use async_std::fs::File;
use futures::AsyncReadExt;
use futures_lite::StreamExt;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

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
pub struct LocalAssets {
    root: Box<Path>,
}

impl LocalAssets {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf().into_boxed_path(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn with_prefix<'a>(&self, path: &'a Path) -> Cow<'a, Path> {
        match path.strip_prefix(&self.root) {
            Ok(p) => Cow::Owned(self.root.join(p)),
            Err(_) => Cow::Borrowed(path),
        }
    }
}

impl AssetIo for LocalAssets {
    type Reader = File;
    type Writer = File;

    async fn reader<'a>(&'a self, path: &'a std::path::Path) -> Result<Self::Reader, AssetIoError> {
        let path = self.with_prefix(path);
        let file = File::open(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;

        Ok(file)
    }

    async fn read_dir<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> Result<Box<dyn PathStream>, AssetIoError> {
        let path = self.with_prefix(path);
        let stream = async_std::fs::read_dir(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?
            .filter_map(move |entry| match entry {
                Ok(e) => Some(PathBuf::from(e.path().as_os_str())),
                Err(_) => None,
            });

        Ok(Box::new(stream) as Box<dyn PathStream>)
    }

    async fn is_dir<'a>(&'a self, path: &'a std::path::Path) -> Result<bool, AssetIoError> {
        let path = self.with_prefix(path);
        let metadata = async_std::fs::metadata(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;
        Ok(metadata.is_dir())
    }

    async fn writer<'a>(&'a self, path: &'a std::path::Path) -> Result<File, AssetIoError> {
        let path = self.with_prefix(path);
        let file = File::create(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))?;
        Ok(file)
    }

    async fn create_dir<'a>(&'a self, path: &'a Path) -> Result<(), AssetIoError> {
        async_std::fs::create_dir(path)
            .await
            .map_err(AssetIoError::from)
    }

    async fn create_dir_all<'a>(&'a self, path: &'a Path) -> Result<(), AssetIoError> {
        async_std::fs::create_dir_all(path)
            .await
            .map_err(AssetIoError::from)
    }

    async fn rename<'a>(
        &'a self,
        from: &'a std::path::Path,
        to: &'a std::path::Path,
    ) -> Result<(), AssetIoError> {
        let from = self.with_prefix(from);
        let to = self.with_prefix(to);
        async_std::fs::rename(from.as_ref(), to.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn remove<'a>(&'a self, path: &'a std::path::Path) -> Result<(), AssetIoError> {
        let path = self.with_prefix(path);
        async_std::fs::remove_file(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn remove_dir<'a>(&'a self, path: &'a std::path::Path) -> Result<(), AssetIoError> {
        let path = self.with_prefix(path);
        async_std::fs::remove_dir_all(path.as_ref())
            .await
            .map_err(|e| AssetIoError::from(e))
    }

    async fn exists<'a>(&'a self, path: &'a Path) -> Result<bool, AssetIoError> {
        let path = self.with_prefix(path);
        Ok(path.exists())
    }
}
