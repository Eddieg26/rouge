use super::{AssetFuture, AssetReader, AssetSourceConfig, AssetWatcher, AssetWriter};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum EmbeddedFileData {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}

pub struct EmbeddedEntry {
    path: PathBuf,
    fs: EmbeddedFS,
}

impl AssetReader for EmbeddedEntry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> AssetFuture<'a, usize> {
        let files = self.fs.files.clone();
        let result = async move {
            let files = files.lock().unwrap();
            match files.get(&self.path) {
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
                None => Err(super::AssetIoError::NotFound(self.path.clone())),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize> {
        let files = self.fs.files.clone();
        let result = async move {
            let files = files.lock().unwrap();
            match files.get(&self.path) {
                Some(EmbeddedFileData::Static(data)) => {
                    buf.extend_from_slice(data);
                    Ok(data.len())
                }
                Some(EmbeddedFileData::Dynamic(data)) => {
                    buf.extend_from_slice(data);
                    Ok(data.len())
                }
                None => Err(super::AssetIoError::NotFound(self.path.clone())),
            }
        };

        Box::pin(result)
    }

    fn read_directory(&mut self) -> AssetFuture<Vec<PathBuf>> {
        Box::pin(async move { Err(super::AssetIoError::NotFound(self.path.clone())) })
    }

    fn is_directory(&mut self) -> AssetFuture<bool> {
        Box::pin(async { Ok(false) })
    }
}

impl AssetWriter for EmbeddedEntry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn write<'a>(&'a mut self, data: &'a [u8]) -> AssetFuture<'a, usize> {
        let result = async move {
            let files = self.fs.files.clone();
            let mut files = files.lock().unwrap();
            files.insert(
                self.path.to_path_buf(),
                EmbeddedFileData::Dynamic(data.to_vec()),
            );

            Ok(data.len())
        };

        Box::pin(result)
    }

    fn create_directory<'a>(&'a mut self) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            return Err(super::AssetIoError::NotFound(self.path.to_path_buf()));
        })
    }

    fn remove<'a>(&'a mut self) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut files = self.fs.files.lock().unwrap();
            if files.remove(&self.path).is_some() {
                Ok(())
            } else {
                Err(super::AssetIoError::NotFound(self.path.to_path_buf()))
            }
        })
    }

    fn remove_directory<'a>(&'a mut self) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            return Err(super::AssetIoError::NotFound(self.path.to_path_buf()));
        })
    }

    fn rename<'a>(&'a mut self, to: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut files = self.fs.files.lock().unwrap();
            if let Some(data) = files.remove(&self.path) {
                files.insert(to.to_path_buf(), data);
                Ok(())
            } else {
                Err(super::AssetIoError::NotFound(self.path.to_path_buf()))
            }
        })
    }
}

impl AssetWatcher for EmbeddedEntry {
    fn watch(&self, _: &Path) -> Result<(), super::AssetIoError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedFS {
    files: Arc<Mutex<HashMap<PathBuf, EmbeddedFileData>>>,
}

impl EmbeddedFS {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn embed(&mut self, path: impl AsRef<Path>, data: &'static [u8]) {
        let mut files = self.files.lock().unwrap();
        files.insert(path.as_ref().to_path_buf(), EmbeddedFileData::Static(data));
    }
}

impl AssetSourceConfig for EmbeddedFS {
    fn reader(&self, path: &Path) -> Box<dyn AssetReader> {
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
        };
        Box::new(entry)
    }

    fn writer(&self, path: &Path) -> Box<dyn AssetWriter> {
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
        };
        Box::new(entry)
    }

    fn watcher(&self, path: &Path) -> Box<dyn AssetWatcher> {
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
        };
        Box::new(entry)
    }
}
