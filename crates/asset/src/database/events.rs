use super::{AssetEvent, StartAssetEvent};
use crate::{
    import::registry::ImportError,
    io::{AssetFuture, AssetIoError, AssetSource, SourceId},
};
use ecs::{event::Events, world::action::WorldAction};
use futures::executor::block_on;
use std::path::{Path, PathBuf};

pub enum ImportScan {
    Added(PathBuf),
    Removed(PathBuf),
    Modified(PathBuf),
    Error { path: PathBuf, error: ImportError },
}

impl ImportScan {
    pub fn added(path: impl AsRef<Path>) -> Self {
        Self::Added(path.as_ref().to_path_buf())
    }

    pub fn removed(path: impl AsRef<Path>) -> Self {
        Self::Removed(path.as_ref().to_path_buf())
    }

    pub fn modified(path: impl AsRef<Path>) -> Self {
        Self::Modified(path.as_ref().to_path_buf())
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            ImportScan::Added(path) => path,
            ImportScan::Removed(path) => path,
            ImportScan::Modified(path) => path,
            ImportScan::Error { path, .. } => path,
        }
    }
}

pub struct ImportFolder {
    source: SourceId,
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: PathBuf) -> Self {
        Self {
            source: SourceId::Default,
            path,
        }
    }

    pub fn from_source(source: impl Into<SourceId>, path: PathBuf) -> Self {
        Self {
            source: source.into(),
            path,
        }
    }

    async fn scan_folder<'a>(
        path: &'a Path,
        source: &'a AssetSource,
    ) -> Result<Vec<ImportScan>, AssetIoError> {
        let mut reader = source.reader(path);
        let paths = reader.read_directory().await?;

        let mut imports = Vec::new();
        for path in paths {
            if path.is_dir() {
                imports.extend(Box::pin(Self::scan_folder(&path, source)).await?);
            } else {
                imports.push(Self::scan_file(&path, source).await?);
            }
        }

        Ok(imports)
    }

    async fn scan_file<'a>(
        path: &'a Path,
        source: &'a AssetSource,
    ) -> Result<ImportScan, AssetIoError> {
        todo!()
    }
}

impl WorldAction for ImportFolder {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportFolder {
    fn execute(
        &mut self,
        database: &super::AssetDatabase,
        actions: &ecs::world::action::WorldActions,
    ) {
        if let Some(source) = database.config().source(&self.source) {
            let mut reader = source.reader(&self.path);
            match block_on(Self::scan_folder(&self.path, source)) {
                Ok(imports) => for import in imports {},
                Err(_) => todo!(),
            }
        }
    }
}

pub struct ImportAsset {
    source: SourceId,
    path: PathBuf,
}

impl ImportAsset {
    pub fn new(path: PathBuf) -> Self {
        Self {
            source: SourceId::Default,
            path,
        }
    }

    pub fn from_source(source: impl Into<SourceId>, path: PathBuf) -> Self {
        Self {
            source: source.into(),
            path,
        }
    }
}
