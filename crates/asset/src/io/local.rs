use super::{AssetIoError, AssetReader, AssetWriter};
use async_std::{
    fs::File,
    io::{ReadExt, WriteExt},
    stream::StreamExt,
};
use std::path::PathBuf;

pub struct LocalFS;

pub struct LocalEntry {
    path: PathBuf,
    file: Option<File>,
}

impl AssetReader for LocalEntry {
    fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let file = match &mut self.file {
                Some(file) => file,
                None => {
                    return Err(AssetIoError::from(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Path is not a file",
                    )))
                }
            };

            match file.read(buf).await {
                Ok(len) => Ok(len),
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let file = match &mut self.file {
                Some(file) => file,
                None => {
                    return Err(AssetIoError::from(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Path is not a file",
                    )))
                }
            };

            match file.read_to_end(buf).await {
                Ok(len) => Ok(len),
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

    fn is_directory<'a>(&'a mut self) -> super::AssetFuture<'a, bool> {
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
                Ok(len) => Ok(len),
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
}
