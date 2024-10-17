use super::{AssetIo, AssetReader, AssetWriter};
use async_std::sync::RwLock;
use futures::{AsyncRead, AsyncWrite};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub enum EmbeddedBytes {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}

pub struct EmbeddedReader {
    path: PathBuf,
    fs: EmbeddedFs,
    offset: usize,
}

impl EmbeddedReader {
    pub fn new(path: PathBuf, fs: EmbeddedFs) -> Self {
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
                let end = (len - self.offset).min(buf.len());
                buf.copy_from_slice(&bytes[self.offset..self.offset + end]);
                self.offset += end;
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
        let end = if self.offset < len {
            let end = (len - self.offset).min(buf.len());
            buf.copy_from_slice(&bytes[self.offset..self.offset + end]);
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
    fs: EmbeddedFs,
    bytes: Vec<u8>,
}

impl EmbeddedWriter {
    pub fn new(path: PathBuf, fs: EmbeddedFs) -> Self {
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
pub struct EmbeddedFs {
    assets: Arc<RwLock<HashMap<PathBuf, EmbeddedBytes>>>,
}

impl EmbeddedFs {
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn embed(&self, path: PathBuf, bytes: &'static [u8]) {
        let mut assets = self.assets.write_blocking();
        assets.insert(path, EmbeddedBytes::Static(bytes));
    }
}

impl AssetIo for EmbeddedFs {
    fn reader<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<'a, Box<dyn AssetReader>> {
        Box::pin(async move {
            let reader = EmbeddedReader::new(path.to_path_buf(), self.clone());
            Ok(Box::new(reader) as Box<dyn AssetReader>)
        })
    }

    fn read_dir<'a>(
        &'a self,
        _: &'a std::path::Path,
    ) -> super::AssetFuture<Box<dyn futures::prelude::Stream<Item = PathBuf>>> {
        Box::pin(async move {
            let stream = futures::stream::empty();
            Ok(Box::new(stream) as Box<dyn futures::prelude::Stream<Item = PathBuf>>)
        })
    }

    fn is_dir<'a>(&'a self, _: &'a std::path::Path) -> super::AssetFuture<'a, bool> {
        Box::pin(async { Ok(false) })
    }

    fn writer<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> super::AssetFuture<'a, Box<dyn AssetWriter>> {
        Box::pin(async move {
            let writer = EmbeddedWriter::new(path.to_path_buf(), self.clone());
            Ok(Box::new(writer) as Box<dyn AssetWriter>)
        })
    }

    fn rename<'a>(
        &'a self,
        from: &'a std::path::Path,
        to: &'a std::path::Path,
    ) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut assets = self.assets.write_blocking();
            if let Some(bytes) = assets.remove(from) {
                assets.insert(to.to_path_buf(), bytes);
            }

            Ok(())
        })
    }

    fn remove<'a>(&'a self, path: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async move {
            let mut assets = self.assets.write_blocking();
            assets.remove(path);
            Ok(())
        })
    }

    fn remove_dir<'a>(&'a self, _: &'a std::path::Path) -> super::AssetFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
