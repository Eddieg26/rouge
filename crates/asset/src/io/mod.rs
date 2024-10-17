use futures::{AsyncRead, AsyncWrite, Stream};
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

impl From<bincode::Error> for AssetIoError {
    fn from(value: bincode::Error) -> Self {
        Self::Io(Arc::new(match *value {
            bincode::ErrorKind::Io(error) => error,
            bincode::ErrorKind::InvalidUtf8Encoding(utf8_error) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 encoding: {}", utf8_error),
            ),
            bincode::ErrorKind::InvalidBoolEncoding(_) => todo!(),
            bincode::ErrorKind::InvalidCharEncoding => todo!(),
            bincode::ErrorKind::InvalidTagEncoding(_) => todo!(),
            bincode::ErrorKind::DeserializeAnyNotSupported => todo!(),
            bincode::ErrorKind::SizeLimit => todo!(),
            bincode::ErrorKind::SequenceMustHaveLength => todo!(),
            bincode::ErrorKind::Custom(_) => todo!(),
        }))
    }
}

pub type AssetFuture<'a, T, E = AssetIoError> = Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>;

pub trait AssetReader: AsyncRead + Send + Sync + Unpin {
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> AssetFuture<'a, usize>;
}

pub trait AssetWriter: AsyncWrite + Send + Sync + Unpin {}

pub trait AssetIo: Send + Sync + 'static {
    fn reader<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetReader>>;
    fn read_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<Box<dyn Stream<Item = PathBuf>>>;
    fn is_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, bool>;

    fn writer<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, Box<dyn AssetWriter>>;
    fn rename<'a>(&'a self, from: &'a Path, to: &'a Path) -> AssetFuture<'a, ()>;
    fn remove<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
    fn remove_dir<'a>(&'a self, path: &'a Path) -> AssetFuture<'a, ()>;
}
