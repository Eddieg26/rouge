use std::path::{Path, PathBuf};

pub struct Metadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified: u64,
    pub accessed: u64,
    pub created: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemError {
    NotFound,
    AlreadyExists,
    IsDirectory,
    IsFile,
    Other(String),
}

pub trait FileSystem: Send + Sync + 'static {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError>;
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FileSystemError>;
    fn read_str(&self, path: &Path) -> Result<String, FileSystemError>;
    fn write_str(&self, path: &Path, data: &str) -> Result<(), FileSystemError>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn create_dir(&self, path: &Path) -> Result<(), FileSystemError>;
    fn remove_dir(&self, path: &Path) -> Result<(), FileSystemError>;
    fn remove_file(&self, path: &Path) -> Result<(), FileSystemError>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError>;
    fn copy(&self, from: &Path, to: &Path) -> Result<u64, FileSystemError>;
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError>;
    fn list_recursive(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError>;
    fn metadata(&self, path: &Path) -> Result<Metadata, FileSystemError>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FileSystemError>;
}

pub struct LocalFileSystem {
    root: PathBuf,
}

impl LocalFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl FileSystem for LocalFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when reading file: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when reading file: {}",
                path.display()
            )),
        })
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::write(&path, data).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when writing file: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when writing file: {}",
                path.display()
            )),
        })
    }

    fn read_str(&self, path: &Path) -> Result<String, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read_to_string(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when reading file: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when reading file: {}",
                path.display()
            )),
        })
    }

    fn write_str(&self, path: &Path, data: &str) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::write(&path, data).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when writing file: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when writing file: {}",
                path.display()
            )),
        })
    }

    fn exists(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        let path = self.root.join(path);
        path.is_dir()
    }

    fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::create_dir(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when creating directory: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when creating directory: {}",
                path.display()
            )),
        })
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::remove_dir(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when removing directory: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when removing directory: {}",
                path.display()
            )),
        })
    }

    fn remove_file(&self, path: &Path) -> Result<(), FileSystemError> {
        let path = self.root.join(path);
        std::fs::remove_file(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when removing file: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when removing file: {}",
                path.display()
            )),
        })
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        let from = self.root.join(from);
        let to = self.root.join(to);
        std::fs::rename(&from, &to).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when renaming file: {}",
                from.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when renaming file: {}",
                from.display()
            )),
        })
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<u64, FileSystemError> {
        let from = self.root.join(from);
        let to = self.root.join(to);
        std::fs::copy(&from, &to).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when copying file: {}",
                from.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when copying file: {}",
                from.display()
            )),
        })
    }

    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let path = self.root.join(path);
        std::fs::read_dir(&path)
            .map(|entries| {
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
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound,
                std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                    "Permission denied when listing directory: {}",
                    path.display()
                )),
                _ => FileSystemError::Other(format!(
                    "Unknown error when listing directory: {}",
                    path.display()
                )),
            })
    }

    fn list_recursive(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let path = self.root.join(path);
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileSystemError::NotFound,
            std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                "Permission denied when listing directory: {}",
                path.display()
            )),
            _ => FileSystemError::Other(format!(
                "Unknown error when listing directory: {}",
                path.display()
            )),
        })? {
            let entry = entry.map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound,
                std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                    "Permission denied when listing directory: {}",
                    path.display()
                )),
                _ => FileSystemError::Other(format!(
                    "Unknown error when listing directory: {}",
                    path.display()
                )),
            })?;
            let path = entry.path();
            if path.is_dir() {
                paths.extend(self.list_recursive(&path)?);
            } else {
                paths.push(path.strip_prefix(&self.root).unwrap().to_path_buf());
            }
        }
        Ok(paths)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FileSystemError> {
        let path = self.root.join(path);
        std::fs::metadata(&path)
            .map(|metadata| Metadata {
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                len: metadata.len(),
                modified: metadata.modified().unwrap().elapsed().unwrap().as_secs(),
                accessed: metadata.accessed().unwrap().elapsed().unwrap().as_secs(),
                created: metadata.created().unwrap().elapsed().unwrap().as_secs(),
            })
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound,
                std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                    "Permission denied when getting metadata: {}",
                    path.display()
                )),
                _ => FileSystemError::Other(format!(
                    "Unknown error when getting metadata: {}",
                    path.display()
                )),
            })
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FileSystemError> {
        let path = self.root.join(path);
        std::fs::canonicalize(&path)
            .map(|path| path.strip_prefix(&self.root).unwrap().to_path_buf())
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileSystemError::NotFound,
                std::io::ErrorKind::PermissionDenied => FileSystemError::Other(format!(
                    "Permission denied when canonicalizing path: {}",
                    path.display()
                )),
                _ => FileSystemError::Other(format!(
                    "Unknown error when canonicalizing path: {}",
                    path.display()
                )),
            })
    }
}
