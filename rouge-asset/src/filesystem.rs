use rouge_ecs::{macros::Resource, world::resource::Resource};
use std::path::{Path, PathBuf};
pub struct Metadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified: u64,
    pub accessed: u64,
    pub created: u64,
}

pub type FileSystemError = std::io::Error;

pub trait PathExt {
    fn normalize(&self) -> PathBuf;
}

impl<A: AsRef<str>> PathExt for A {
    fn normalize(&self) -> PathBuf {
        self.as_ref().replace("\\", "/").into()
    }
}

impl PathExt for Path {
    fn normalize(&self) -> PathBuf {
        self.to_str().unwrap().replace("\\", "/").into()
    }
}

#[derive(Resource)]
pub struct FileSystem {
    root: PathBuf,
}

impl FileSystem {}

impl FileSystem {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read(&path)
    }

    pub fn write(&self, path: &Path, data: &[u8]) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::write(&path, data)
    }

    pub fn read_str(&self, path: &Path) -> Result<String, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read_to_string(&path)
    }

    pub fn write_str(&self, path: &Path, data: &str) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::write(&path, data)
    }

    pub fn exists(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.exists()
    }

    pub fn is_file(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.is_file()
    }

    pub fn is_dir(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.is_dir()
    }

    pub fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::create_dir(&path)
    }

    pub fn create_dir_all(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::create_dir_all(&path)
    }

    pub fn remove_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::remove_dir(&path)
    }

    pub fn remove_file(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::remove_file(&path)
    }

    pub fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        let from = self.root.join(from);
        let to = self.root.join(to);
        std::fs::rename(&from, &to)
    }

    pub fn copy(&self, from: &Path, to: &Path) -> Result<u64, FileSystemError> {
        let from = self.root.join(from);
        let to = self.root.join(to);
        std::fs::copy(&from, &to)
    }

    pub fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read_dir(&path).map(|entries| {
            entries
                .filter_map(|entry| {
                    entry.ok().and_then(|entry| {
                        entry
                            .path()
                            .strip_prefix(&self.root)
                            .ok()
                            .map(|path| path.to_path_buf())
                    })
                })
                .collect()
        })
    }

    pub fn list_recursive(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let path = self.root.join(path);
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let path = path.strip_prefix(&self.root).unwrap().to_path_buf();
                paths.extend(self.list_recursive(&path)?);
            } else {
                paths.push(path.strip_prefix(&self.root).unwrap().to_path_buf());
            }
        }
        Ok(paths)
    }

    pub fn metadata(&self, path: &Path) -> Result<Metadata, FileSystemError> {
        let path = self.root.join(path);
        std::fs::metadata(&path).map(|metadata| Metadata {
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            len: metadata.len(),
            modified: metadata.modified().unwrap().elapsed().unwrap().as_secs(),
            accessed: metadata.accessed().unwrap().elapsed().unwrap().as_secs(),
            created: metadata.created().unwrap().elapsed().unwrap().as_secs(),
        })
    }

    pub fn canonicalize(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        let path = self.root.join(path);
        std::fs::canonicalize(&path)
    }
}
