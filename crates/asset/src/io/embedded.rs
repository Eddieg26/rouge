use crate::asset::{Asset, AssetMetadata, AssetRef, Settings};

use super::{AssetIo, AssetIoError, AssetReader, AssetWriter, PathExt, PathStream};
use async_std::sync::RwLock;
use futures::{AsyncRead, AsyncWrite};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub enum EmbeddedBytes {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}

pub struct EmbeddedReader {
    path: PathBuf,
    fs: EmbeddedAssets,
    offset: usize,
}

impl EmbeddedReader {
    pub fn new(path: PathBuf, fs: EmbeddedAssets) -> Self {
        Self {
            path,
            fs,
            offset: 0,
        }
    }
}

impl AssetReader for EmbeddedReader {
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> super::AssetFuture<'a, usize> {
        Box::pin(async move {
            let assets = self.fs.assets.read().await;
            let asset = assets
                .get(&self.path)
                .ok_or(super::AssetIoError::NotFound(self.path.to_path_buf()))?;
            let bytes = match asset {
                EmbeddedBytes::Static(bytes) => bytes,
                EmbeddedBytes::Dynamic(bytes) => bytes.as_slice(),
            };
            let len = bytes.len();
            if self.offset < len {
                buf.extend(&bytes[self.offset..]);
                let end = len - self.offset;
                Ok(end)
            } else {
                Ok(0)
            }
        })
    }
}

impl AsyncRead for EmbeddedReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let assets = self.fs.assets.read_blocking();
        let asset = assets.get(&self.path).ok_or(std::io::ErrorKind::NotFound)?;
        let bytes = match asset {
            EmbeddedBytes::Static(bytes) => bytes,
            EmbeddedBytes::Dynamic(bytes) => bytes.as_slice(),
        };
        let len = bytes.len();
        let buf_len = buf.len();
        let end = if self.offset < len {
            let end = (len - self.offset).min(buf.len());
            buf[..len.min(buf_len)].copy_from_slice(&bytes[self.offset..self.offset + end]);
            end
        } else {
            0
        };

        std::mem::drop(assets);

        self.offset += end;
        std::task::Poll::Ready(Ok(end))
    }
}

pub struct EmbeddedWriter {
    path: PathBuf,
    fs: EmbeddedAssets,
    bytes: Vec<u8>,
}

impl EmbeddedWriter {
    pub fn new(path: PathBuf, fs: EmbeddedAssets) -> Self {
        Self {
            path,
            fs,
            bytes: Vec::new(),
        }
    }
}

impl AssetWriter for EmbeddedWriter {}

impl AsyncWrite for EmbeddedWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.bytes.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut assets = self.fs.assets.write_blocking();
        assets.insert(
            self.path.clone(),
            EmbeddedBytes::Dynamic(self.bytes.clone()),
        );
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

#[derive(Clone)]
pub struct EmbeddedAssets {
    assets: Arc<RwLock<HashMap<PathBuf, EmbeddedBytes>>>,
}

impl EmbeddedAssets {
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn embed<A: Asset, S: Settings>(
        &self,
        id: AssetRef<A>,
        path: impl AsRef<Path>,
        bytes: &'static [u8],
        settings: S,
    ) {
        let meta_path = path.as_ref().append_ext("meta");
        let metadata = AssetMetadata::<A, S>::new(id.into(), settings);
        let meta_bytes = ron::to_string(&metadata).unwrap().into_bytes();

        let mut assets = self.assets.write_blocking();
        assets.insert(path.as_ref().to_path_buf(), EmbeddedBytes::Static(bytes));
        assets.insert(meta_path, EmbeddedBytes::Dynamic(meta_bytes));
    }
}

impl AssetIo for EmbeddedAssets {
    type Reader = EmbeddedReader;
    type Writer = EmbeddedWriter;

    async fn reader<'a>(&'a self, path: &'a std::path::Path) -> Result<Self::Reader, AssetIoError> {
        let reader = EmbeddedReader::new(path.to_path_buf(), self.clone());
        Ok(reader)
    }

    async fn read_dir<'a>(
        &'a self,
        _: &'a std::path::Path,
    ) -> Result<Box<dyn PathStream>, AssetIoError> {
        let assets = self.assets.read().await;
        let paths = assets.keys().cloned().collect::<Vec<_>>();
        Ok(Box::new(futures::stream::iter(paths)))
    }

    async fn is_dir<'a>(&'a self, _: &'a std::path::Path) -> Result<bool, AssetIoError> {
        Ok(false)
    }

    async fn create_dir<'a>(&'a self, _: &'a std::path::Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn create_dir_all<'a>(&'a self, _: &'a std::path::Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn writer<'a>(&'a self, path: &'a std::path::Path) -> Result<Self::Writer, AssetIoError> {
        let writer = EmbeddedWriter::new(path.to_path_buf(), self.clone());
        Ok(writer)
    }

    async fn rename<'a>(
        &'a self,
        from: &'a std::path::Path,
        to: &'a std::path::Path,
    ) -> Result<(), AssetIoError> {
        let mut assets = self.assets.write_blocking();
        if let Some(bytes) = assets.remove(from) {
            assets.insert(to.to_path_buf(), bytes);
        }

        Ok(())
    }

    async fn remove<'a>(&'a self, path: &'a std::path::Path) -> Result<(), AssetIoError> {
        let mut assets = self.assets.write_blocking();
        assets.remove(path);
        Ok(())
    }

    async fn remove_dir<'a>(&'a self, _: &'a std::path::Path) -> Result<(), AssetIoError> {
        Ok(())
    }

    async fn exists<'a>(&'a self, path: &'a std::path::Path) -> Result<bool, AssetIoError> {
        let assets = self.assets.read().await;
        Ok(assets.contains_key(path))
    }
}

#[macro_export]
macro_rules! embed_asset {
    ($assets:expr, $id:expr, $path:expr, $settings:expr) => {
        $assets.embed($id, $path, include_bytes!($path), $settings)
    };
}
