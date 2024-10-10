use super::{AssetIoError, AssetReader, AssetSourceConfig, AssetWatcher, AssetWriter};
use async_std::{fs::File, stream::StreamExt};
use futures::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;

pub struct LocalFS;

pub struct LocalEntry {
    pub path: PathBuf,
    pub file: Option<File>,
}

impl LocalEntry {
    pub fn new(path: PathBuf) -> Self {
        Self { path, file: None }
    }
}

impl AssetReader for LocalEntry {
    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match self.file.take() {
                Some(file) => file,
                None => match File::open(&self.path).await {
                    Ok(file) => file,
                    Err(error) => return Err(super::AssetIoError::from(error)),
                },
            };

            match file.read(buf).await {
                Ok(len) => {
                    self.file = Some(file);
                    Ok(len)
                }
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match self.file.take() {
                Some(file) => file,
                None => match File::open(&self.path).await {
                    Ok(file) => file,
                    Err(error) => return Err(super::AssetIoError::from(error)),
                },
            };

            match file.read_to_end(buf).await {
                Ok(len) => {
                    self.file = Some(file);
                    Ok(len)
                }
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn read_directory<'a>(&'a mut self) -> super::AssetFuture<'a, Vec<PathBuf>> {
        let result = async move {
            let entries = match async_std::fs::read_dir(&self.path).await {
                Ok(entries) => entries,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            let mut paths = vec![];
            let mut stream = entries.map(|entry| entry.map(|entry| entry.path()));
            while let Some(path) = stream.next().await {
                paths.push(path?.as_os_str().into());
            }
            Ok(paths)
        };

        Box::pin(result)
    }

    fn is_directory<'a>(&'a self) -> super::AssetFuture<'a, bool> {
        let result = async move {
            let metadata = match async_std::fs::metadata(&self.path).await {
                Ok(metadata) => metadata,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            Ok(metadata.is_dir())
        };

        Box::pin(result)
    }
}

impl AssetWriter for LocalEntry {
    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn write<'a>(&'a mut self, data: &'a [u8]) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match File::create(&self.path).await {
                Ok(file) => file,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            match file.write(data).await {
                Ok(len) => {
                    self.file = Some(file);
                    Ok(len)
                }
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn create_directory<'a>(&'a mut self) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::create_dir(&self.path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn remove<'a>(&'a mut self) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::remove_file(&self.path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn remove_directory<'a>(&'a mut self) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::remove_dir(&self.path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn rename<'a>(&'a mut self, to: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::rename(&self.path, to).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn flush<'a>(&'a mut self) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match self.file.take() {
                Some(mut file) => file.flush().await.map_err(super::AssetIoError::from),
                None => Ok(()),
            }
        };

        Box::pin(result)
    }
}

pub struct LocalWatcher {
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl LocalWatcher {
    pub fn new() -> Self {
        Self { watcher: None }
    }
}

impl AssetWatcher for LocalWatcher {
    fn watch(&self, path: &std::path::Path) -> Result<(), AssetIoError> {
        Ok(())
    }
}

impl AssetSourceConfig for LocalFS {
    fn reader(&self, path: &std::path::Path) -> Box<dyn AssetReader> {
        Box::new(LocalEntry::new(path.to_path_buf()))
    }

    fn writer(&self, path: &std::path::Path) -> Box<dyn AssetWriter> {
        Box::new(LocalEntry::new(path.to_path_buf()))
    }

    fn watcher(&self, path: &std::path::Path) -> Box<dyn super::AssetWatcher> {
        Box::new(LocalWatcher::new())
    }
}
