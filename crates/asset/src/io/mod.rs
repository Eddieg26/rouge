use futures_lite::{AsyncRead, AsyncWrite, Stream};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use thiserror::Error;

pub mod cache;
pub mod embedded;
pub mod local;
pub mod source;
pub mod vfs;

#[derive(Clone, Error, Debug)]
pub enum AssetIoError {
    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("{0}")]
    Io(Arc<std::io::Error>),

    #[error("Http error {0}")]
    Http(u16),
}

impl From<std::io::Error> for AssetIoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(Arc::new(value))
    }
}

impl From<std::io::ErrorKind> for AssetIoError {
    fn from(value: std::io::ErrorKind) -> Self {
        Self::Io(Arc::new(std::io::Error::from(value)))
    }
}

impl From<bincode::Error> for AssetIoError {
    fn from(value: bincode::Error) -> Self {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidData, value);
        Self::Io(Arc::new(error))
    }
}

impl From<ron::Error> for AssetIoError {
    fn from(value: ron::Error) -> Self {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidData, value);
        Self::Io(Arc::new(error))
    }
}

impl From<ron::error::SpannedError> for AssetIoError {
    fn from(value: ron::error::SpannedError) -> Self {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidData, value);
        Self::Io(Arc::new(error))
    }
}

pub type BoxedFuture<'a, T, E = AssetIoError> = Box<dyn Future<Output = Result<T, E>> + 'a>;
pub type AssetFuture<'a, T, E = AssetIoError> = Pin<BoxedFuture<'a, T, E>>;

pub trait AssetReader: AsyncRead + Send + Sync + Unpin {
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize>;
}

pub trait AssetWriter: AsyncWrite + Send + Sync + Unpin {}

pub trait PathStream: Stream<Item = PathBuf> + Send + Unpin {}

impl<T: Stream<Item = PathBuf> + Send + Unpin> PathStream for T {}

pub trait AssetIo: Send + Sync + 'static {
    type Reader: AssetReader;
    type Writer: AssetWriter;

    fn reader<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Self::Reader, AssetIoError>> + 'a;
    fn read_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Box<dyn PathStream>, AssetIoError>> + 'a;
    fn is_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<bool, AssetIoError>> + 'a;
    fn writer<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Self::Writer, AssetIoError>> + 'a;
    fn create_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<(), AssetIoError>> + 'a;
    fn create_dir_all<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<(), AssetIoError>> + 'a;
    fn rename<'a>(
        &'a self,
        from: &'a Path,
        to: &'a Path,
    ) -> impl Future<Output = Result<(), AssetIoError>> + 'a;
    fn remove<'a>(&'a self, path: &'a Path) -> impl Future<Output = Result<(), AssetIoError>> + 'a;
    fn remove_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<(), AssetIoError>> + 'a;
    fn exists<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<bool, AssetIoError>> + 'a;
}

pub trait ErasedAssetIo: downcast_rs::Downcast + Send + Sync + 'static {
    fn reader<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetReader>>;
    fn read_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn PathStream>>;
    fn is_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool>;
    fn writer<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetWriter>>;
    fn create_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn create_dir_all<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()>;
    fn remove<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn remove_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn exists<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool>;
}
downcast_rs::impl_downcast!(ErasedAssetIo);

impl<T: AssetIo> ErasedAssetIo for T {
    fn reader<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetReader>> {
        Box::pin(async {
            let reader = self.reader(path).await?;
            Ok(Box::new(reader) as Box<dyn AssetReader>)
        })
    }

    fn read_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn PathStream>> {
        Box::pin(async { self.read_dir(path).await })
    }

    fn is_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool> {
        Box::pin(async { self.is_dir(path).await })
    }

    fn writer<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetWriter>> {
        Box::pin(async {
            let writer = self.writer(path).await?;
            Ok(Box::new(writer) as Box<dyn AssetWriter>)
        })
    }

    fn create_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async { self.create_dir(path).await })
    }

    fn create_dir_all<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async { self.create_dir_all(path).await })
    }

    fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async { self.rename(from, to).await })
    }

    fn remove<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async { self.remove(path).await })
    }

    fn remove_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()> {
        Box::pin(async { self.remove(path).await })
    }

    fn exists<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool> {
        Box::pin(async { self.exists(path).await })
    }
}

pub trait PathExt {
    fn ext(&self) -> Option<&str>;
    fn append_ext(&self, ext: &str) -> PathBuf;
    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf;
    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path;
}

impl<T: AsRef<Path>> PathExt for T {
    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }
    fn append_ext(&self, ext: &str) -> PathBuf {
        let path = self.as_ref().to_path_buf();
        format!("{}.{}", path.display(), ext).into()
    }
    fn with_prefix(&self, prefix: impl AsRef<Path>) -> PathBuf {
        match self.as_ref().starts_with(prefix.as_ref()) {
            true => self.as_ref().to_path_buf(),
            false => prefix.as_ref().join(self.as_ref()),
        }
    }
    fn without_prefix(&self, prefix: impl AsRef<Path>) -> &Path {
        let path = self.as_ref();
        let prefix = prefix.as_ref();
        if path.starts_with(prefix) {
            path.strip_prefix(prefix).unwrap()
        } else {
            path
        }
    }
}
