use super::{AssetReader, AssetWriter};
use async_std::{
    fs::File,
    io::{ReadExt, WriteExt},
    stream::StreamExt,
};
use std::path::PathBuf;

pub struct LocalFS;

pub struct LocalFile;

impl AssetReader for LocalFile {
    fn read<'a>(
        &'a mut self,
        path: &'a std::path::Path,
        buf: &'a mut [u8],
    ) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match File::open(path).await {
                Ok(file) => file,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            match file.read(buf).await {
                Ok(len) => Ok(len),
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(
        &mut self,
        path: &'a std::path::Path,
        buf: &'a mut Vec<u8>,
    ) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match File::open(path).await {
                Ok(file) => file,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            match file.read_to_end(buf).await {
                Ok(len) => Ok(len),
                Err(error) => return Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn read_directory<'a>(
        &'a mut self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<'a, Vec<PathBuf>> {
        let result = async move {
            let entries = match async_std::fs::read_dir(path).await {
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

    fn is_directory<'a>(&'a mut self, path: &'a std::path::Path) -> super::AssetFuture<'a, bool> {
        let result = async move {
            let metadata = match async_std::fs::metadata(path).await {
                Ok(metadata) => metadata,
                Err(error) => return Err(super::AssetIoError::from(error)),
            };
            Ok(metadata.is_dir())
        };

        Box::pin(result)
    }
}

impl AssetWriter for LocalFile {
    fn write<'a>(
        &'a mut self,
        path: &'a std::path::Path,
        data: &'a [u8],
    ) -> super::AssetFuture<'a, usize> {
        let result = async move {
            let mut file = match File::create(path).await {
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

    fn create_directory<'a>(&'a mut self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::create_dir(path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn remove<'a>(&'a mut self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::remove_file(path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn remove_directory<'a>(&'a mut self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::remove_dir(path).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }

    fn rename<'a>(
        &'a mut self,
        from: &'a std::path::Path,
        to: &'a std::path::Path,
    ) -> super::AssetFuture<'a, ()> {
        let result = async move {
            match async_std::fs::rename(from, to).await {
                Ok(()) => Ok(()),
                Err(error) => Err(super::AssetIoError::from(error)),
            }
        };

        Box::pin(result)
    }
}
