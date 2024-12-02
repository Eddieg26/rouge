use super::{AssetIoError, AssetReader, AssetWriter, FileSystem, PathExt};
use crate::asset::{Asset, AssetMetadata, AssetRef, Settings};
use async_std::sync::RwLock;
use futures_lite::{AsyncRead, AsyncWrite};
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub enum EmbeddedData {
    Static(&'static [u8]),
    Dynamic(Arc<[u8]>),
}

impl std::ops::Deref for EmbeddedData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            EmbeddedData::Static(bytes) => bytes,
            EmbeddedData::Dynamic(bytes) => &bytes,
        }
    }
}

pub struct EmbeddedReader {
    data: EmbeddedData,
    position: u64,
}

impl EmbeddedReader {
    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

impl Read for EmbeddedReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len().min((self.size() - self.position) as usize);
        if len > 0 {
            buf[..len]
                .copy_from_slice(&self.data[self.position as usize..self.position as usize + len]);
        }

        self.position += len as u64;
        Ok(len)
    }
}

impl Seek for EmbeddedReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(offset) => self.position = offset,
            std::io::SeekFrom::End(offset) => {
                self.position = (self.position as i64 + offset) as u64
            }
            std::io::SeekFrom::Current(offset) => {
                self.position = (self.data.len() as i64 + offset) as u64
            }
        }

        Ok(self.position)
    }
}

impl AsyncRead for EmbeddedReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let len = buf.len().min((self.size() - self.position) as usize);
        if len > 0 {
            buf[..len]
                .copy_from_slice(&self.data[self.position as usize..self.position as usize + len]);
        }

        self.position += len as u64;
        std::task::Poll::Ready(Ok(len))
    }
}

impl AssetReader for EmbeddedReader {
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> super::AssetFuture<'a, usize> {
        Box::pin(async move {
            let len = self.data.len();
            if self.position < len as u64 {
                buf.extend(&self.data[self.position as usize..]);
                let end = len - self.position as usize;
                Ok(end)
            } else {
                Ok(0)
            }
        })
    }
}

pub struct EmbeddedWriter {
    data: Cursor<Vec<u8>>,
    path: PathBuf,
    fs: Handle,
}

impl Write for EmbeddedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.data.flush()
    }
}

impl Seek for EmbeddedWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.data.seek(pos)
    }
}

impl AsyncWrite for EmbeddedWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let size = self.data.write(buf)?;
        std::task::Poll::Ready(Ok(size))
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.data.flush()?;
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl AssetWriter for EmbeddedWriter {}

impl Drop for EmbeddedWriter {
    fn drop(&mut self) {
        let mut fs = self.fs.write_arc_blocking();
        let data = std::mem::take(self.data.get_mut());

        let file = EmbeddedData::Dynamic(Arc::from(data));
        fs.entries.insert(self.path.clone(), file);
    }
}

#[derive(Default)]
pub struct EmbeddedAssets {
    entries: HashMap<PathBuf, EmbeddedData>,
}

type Handle = Arc<RwLock<EmbeddedAssets>>;

pub struct EmbeddedFs {
    root: Box<Path>,
    fs: Handle,
}

impl EmbeddedFs {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf().into_boxed_path(),
            fs: Handle::default(),
        }
    }

    pub fn embed<A: Asset, S: Settings>(
        &self,
        id: AssetRef<A>,
        path: impl AsRef<Path>,
        asset: &'static [u8],
        settings: S,
    ) -> AssetMetadata<A, S> {
        let metadata = AssetMetadata::<A, S>::new(id.into(), settings);
        let metabytes = match ron::to_string(&metadata) {
            Ok(metabytes) => metabytes,
            Err(err) => panic!("Failed to serialize metadata: {}", err),
        };
        let path = path.as_ref().to_path_buf();

        let mut fs = self.fs.write_blocking();
        fs.entries.insert(path.clone(), EmbeddedData::Static(asset));
        fs.entries.insert(
            path.append_ext("meta"),
            EmbeddedData::Dynamic(metabytes.into_bytes().into()),
        );
        metadata
    }
}

impl std::fmt::Display for EmbeddedFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fs = self.fs.read_blocking();
        let mut paths = fs.entries.keys().collect::<Vec<_>>();
        paths.sort();
        f.debug_list().entries(paths).finish()
    }
}

impl FileSystem for EmbeddedFs {
    type Reader = EmbeddedReader;
    type Writer = EmbeddedWriter;

    fn root(&self) -> &Path {
        &self.root
    }

    async fn reader(&self, path: &Path) -> Result<Self::Reader, AssetIoError> {
        let fs = self.fs.read().await;
        match fs.entries.get(path).cloned() {
            Some(data) => Ok(EmbeddedReader { data, position: 0 }),
            None => Err(AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    async fn read_dir(&self, _: &Path) -> Result<Box<dyn super::PathStream>, AssetIoError> {
        let fs = self.fs.read().await;
        let paths = fs.entries.keys().cloned().collect::<Vec<_>>();
        Ok(Box::new(futures::stream::iter(paths)))
    }

    async fn is_dir(&self, path: &Path) -> Result<bool, AssetIoError> {
        let fs = self.fs.read().await;
        match fs.entries.contains_key(path) {
            true => Ok(false),
            false => Err(AssetIoError::NotFound(path.to_path_buf())),
        }
    }

    async fn writer(&self, path: &Path) -> Result<Self::Writer, AssetIoError> {
        let writer = EmbeddedWriter {
            data: Cursor::new(vec![]),
            path: path.to_path_buf(),
            fs: self.fs.clone(),
        };

        Ok(writer)
    }

    async fn create_dir(&self, _: &Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn create_dir_all(&self, _: &Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), AssetIoError> {
        let mut fs = self.fs.write().await;
        match fs.entries.remove(from) {
            Some(entry) => match fs.entries.contains_key(to) {
                true => return Err(AssetIoError::from(std::io::ErrorKind::AlreadyExists)),
                false => {
                    fs.entries.insert(to.to_path_buf(), entry);
                    Ok(())
                }
            },
            None => Err(AssetIoError::NotFound(from.to_path_buf())),
        }
    }

    async fn remove(&self, _: &Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn remove_dir(&self, _: &Path) -> Result<(), AssetIoError> {
        Err(AssetIoError::from(std::io::ErrorKind::Unsupported))
    }

    async fn exists(&self, path: &Path) -> Result<bool, AssetIoError> {
        let fs = self.fs.read().await;
        Ok(fs.entries.contains_key(path))
    }
}

#[macro_export]
macro_rules! embed_asset {
    ($assets:expr, $id:expr, $path:expr, $settings:expr) => {
        $assets.embed($id, $path, include_bytes!($path), $settings)
    };
}
