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
    file: Option<EmbeddedFileData>,
    offset: usize,
}

impl AssetReader for EmbeddedEntry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn read<'a>(&'a mut self, amount: usize) -> AssetFuture<'a, &'a [u8]> {
        let result = async move {
            match &self.file {
                Some(EmbeddedFileData::Static(data)) => {
                    let end = (self.offset + amount).min(data.len());
                    let slice = &data[self.offset..end];
                    self.offset += slice.len();
                    Ok(slice)
                }
                Some(EmbeddedFileData::Dynamic(data)) => {
                    let end = (self.offset + amount).min(data.len());
                    let slice = &data[self.offset..end];
                    self.offset += slice.len();
                    Ok(slice)
                }
                None => Err(super::AssetIoError::NotFound(self.path.clone())),
            }
        };

        Box::pin(result)
    }

    fn read_to_end<'a>(&'a mut self) -> AssetFuture<'a, &'a [u8]> {
        let result = async move {
            match &self.file {
                Some(EmbeddedFileData::Static(data)) => Ok(*data),
                Some(EmbeddedFileData::Dynamic(data)) => Ok(data.as_slice()),
                None => Err(super::AssetIoError::NotFound(self.path.clone())),
            }
        };

        Box::pin(result)
    }

    fn read_directory(&mut self) -> AssetFuture<Vec<PathBuf>> {
        Box::pin(async move { Err(super::AssetIoError::NotFound(self.path.clone())) })
    }

    fn is_directory(&self) -> AssetFuture<bool> {
        Box::pin(async { Ok(false) })
    }

    fn exists<'a>(&'a self) -> AssetFuture<'a, bool> {
        let files = self.fs.files.lock().unwrap();
        Box::pin(async move { Ok(files.contains_key(&self.path)) })
    }

    fn flush<'a>(&'a mut self) -> AssetFuture<'a, Vec<u8>> {
        let result = async move {
            match self.file.take() {
                Some(EmbeddedFileData::Static(data)) => {
                    let mut files = self.fs.files.lock().unwrap();
                    files.insert(self.path.clone(), EmbeddedFileData::Static(&data));
                    Ok(Vec::from(data))
                }
                Some(EmbeddedFileData::Dynamic(data)) => {
                    let mut files = self.fs.files.lock().unwrap();
                    files.insert(self.path.clone(), EmbeddedFileData::Dynamic(data.clone()));
                    Ok(data)
                }
                None => Err(super::AssetIoError::NotFound(self.path.clone())),
            }
        };

        Box::pin(result)
    }
}

impl AssetWriter for EmbeddedEntry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn write<'a>(&'a mut self, data: &'a [u8]) -> AssetFuture<'a, usize> {
        let result = async move {
            match &mut self.file {
                Some(EmbeddedFileData::Static(_)) => {
                    self.file = Some(EmbeddedFileData::Dynamic(Vec::from(data)));
                }
                Some(EmbeddedFileData::Dynamic(vec)) => {
                    vec.extend_from_slice(data);
                }
                None => {
                    self.file = Some(EmbeddedFileData::Dynamic(Vec::from(data)));
                }
            }

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

    fn flush<'a>(&'a mut self) -> AssetFuture<'a, ()> {
        Box::pin(async { Ok(()) })
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
        let mut files = self.files.lock().unwrap();
        let file = files.remove(path);
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
            file,
            offset: 0,
        };

        Box::new(entry)
    }

    fn writer(&self, path: &Path) -> Box<dyn AssetWriter> {
        let mut files = self.files.lock().unwrap();
        let file = files.remove(path);
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
            file,
            offset: 0,
        };

        Box::new(entry)
    }

    fn watcher(&self, path: &Path) -> Box<dyn AssetWatcher> {
        let mut files = self.files.lock().unwrap();
        let file = files.remove(path);
        let entry = EmbeddedEntry {
            path: path.to_path_buf(),
            fs: self.clone(),
            file,
            offset: 0,
        };

        Box::new(entry)
    }
}
