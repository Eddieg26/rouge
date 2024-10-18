use super::{AssetIo, AssetIoError, AssetReader, AssetWriter};
use async_std::fs::File;
use futures::{AsyncReadExt, StreamExt};
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
    fn reader<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<'a, Box<dyn AssetReader>> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            let file = File::open(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))?;
            Ok(Box::new(file) as Box<dyn AssetReader>)
        })
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<Box<dyn futures::prelude::Stream<Item = std::path::PathBuf>>> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            let stream = async_std::fs::read_dir(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))?
                .filter_map(|entry| async move {
                    match entry {
                        Ok(e) => Some(PathBuf::from(e.path().as_os_str())),
                        Err(_) => None,
                    }
                });
            Ok(Box::new(stream)
                as Box<
                    dyn futures::prelude::Stream<Item = std::path::PathBuf>,
                >)
        })
    }

    fn is_dir<'a>(&'a self, path: &'a std::path::Path) -> super::AssetFuture<'a, bool> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            let metadata = async_std::fs::metadata(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))?;
            Ok(metadata.is_dir())
        })
    }

    fn writer<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<'a, Box<dyn AssetWriter>> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            let file = File::create(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))?;
            Ok(Box::new(file) as Box<dyn AssetWriter>)
        })
    }

    fn create_dir<'a>(&'a self, path: &'a Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            async_std::fs::create_dir(path)
                .await
                .map_err(AssetIoError::from)
        })
    }

    fn create_dir_all<'a>(&'a self, path: &'a Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            async_std::fs::create_dir_all(path)
                .await
                .map_err(AssetIoError::from)
        })
    }

    fn rename<'a>(
        &'a self,
        from: &'a std::path::Path,
        to: &'a std::path::Path,
    ) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            let from = self.with_prefix(from);
            let to = self.with_prefix(to);
            async_std::fs::rename(from.as_ref(), to.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))
        })
    }

    fn remove<'a>(&'a self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            async_std::fs::remove_file(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))
        })
    }

    fn remove_dir<'a>(&'a self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            let path = self.with_prefix(path);
            async_std::fs::remove_dir_all(path.as_ref())
                .await
                .map_err(|e| AssetIoError::from(e))
        })
    }
}
