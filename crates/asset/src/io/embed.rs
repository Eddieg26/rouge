use super::{AssetFuture, AssetReader, AssetSourceConfig, AssetWatcher, AssetWriter};
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum EmbeddedFileData {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}

impl AssetReader for EmbeddedFS {
    fn read<'a>(&'a mut self, path: &'a Path, buf: &'a mut [u8]) -> AssetFuture<'a, usize> {
        let files = self.files.clone();
        let path = path.to_path_buf();
        let result = async move {
            let files = files.lock().unwrap();
            match files.get(path.as_os_str()) {
                Some(EmbeddedFileData::Static(data)) => {
                    let len = data.len().min(buf.len());
                    println!("Copying {} bytes", len);
                    buf[..len].copy_from_slice(&data[..len]);
                    Ok(len)
                }
                Some(EmbeddedFileData::Dynamic(data)) => {
                    let len = data.len().min(buf.len());
                    buf[..len].copy_from_slice(&data[..len]);
                    Ok(len)
                }
                None => Err(super::AssetIoError::NotFound(path.to_path_buf())),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(&mut self, path: &'a Path, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize> {
        let files = self.files.clone();
        let path = path.to_path_buf();
        let result = async move {
            let files = files.lock().unwrap();
            match files.get(path.as_os_str()) {
                Some(EmbeddedFileData::Static(data)) => {
                    buf.extend_from_slice(data);
                    Ok(data.len())
                }
                Some(EmbeddedFileData::Dynamic(data)) => {
                    buf.extend_from_slice(data);
                    Ok(data.len())
                }
                None => Err(super::AssetIoError::NotFound(path.to_path_buf())),
            }
        };

        Box::pin(result)
    }

    fn read_directory(&mut self, path: &Path) -> AssetFuture<Vec<PathBuf>> {
        let path = path.to_path_buf();
        Box::pin(async move { Err(super::AssetIoError::NotFound(path)) })
    }

    fn is_directory(&mut self, _: &Path) -> AssetFuture<bool> {
        Box::pin(async { Ok(false) })
    }
}

impl AssetWriter for EmbeddedFS {
    fn write<'a>(&'a mut self, path: &'a Path, data: &'a [u8]) -> AssetFuture<'a, usize> {
        let files = self.files.clone();
        let path = path.to_path_buf();
        let result = async move {
            let mut files = files.lock().unwrap();
            files.insert(
                path.as_os_str().to_owned(),
                EmbeddedFileData::Dynamic(data.to_vec()),
            );

            Ok(data.len())
        };

        Box::pin(result)
    }

    fn create_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let path = path.to_path_buf();
            return Err(super::AssetIoError::NotFound(path));
        })
    }

    fn remove<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut files = self.files.lock().unwrap();
            if files.remove(path.as_os_str()).is_some() {
                Ok(())
            } else {
                Err(super::AssetIoError::NotFound(path.to_path_buf()))
            }
        })
    }

    fn remove_directory<'a>(&'a mut self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let path = path.to_path_buf();
            return Err(super::AssetIoError::NotFound(path));
        })
    }

    fn rename<'a>(&'a mut self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut files = self.files.lock().unwrap();
            if let Some(data) = files.remove(from.as_os_str()) {
                files.insert(to.as_os_str().to_owned(), data);
                Ok(())
            } else {
                Err(super::AssetIoError::NotFound(from.to_path_buf()))
            }
        })
    }
}

impl AssetWatcher for EmbeddedFS {
    fn watch(&self, _: &Path) -> Result<(), super::AssetIoError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedFS {
    files: Arc<Mutex<HashMap<OsString, EmbeddedFileData>>>,
}

impl EmbeddedFS {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn embed(&mut self, path: impl AsRef<Path>, data: &'static [u8]) {
        let mut files = self.files.lock().unwrap();
        files.insert(
            path.as_ref().as_os_str().to_owned(),
            EmbeddedFileData::Static(data),
        );
    }
}

impl AssetSourceConfig for EmbeddedFS {
    fn reader(&self) -> Box<dyn AssetReader> {
        Box::new(self.clone())
    }

    fn writer(&self) -> Box<dyn AssetWriter> {
        Box::new(self.clone())
    }

    fn watcher(&self) -> Box<dyn AssetWatcher> {
        Box::new(self.clone())
    }
}
