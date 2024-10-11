use super::{AssetDatabase, AssetEvent, StartAssetEvent};
use crate::{
    asset::{Asset, AssetId, AssetKind, AssetSettings, Settings},
    import::registry::ImportError,
    io::{AssetSource, PathExt, SourceId},
    library::SourceAsset,
};
use ecs::{
    event::Events,
    task::AsyncTaskPool,
    world::action::{WorldAction, WorldActions},
};
use futures::{
    executor::{block_on, LocalPool},
    future::join_all,
    task::SpawnExt,
    TryFutureExt,
};
use hashbrown::{hash_set::Difference, DefaultHashBuilder, HashSet};
use std::path::{Path, PathBuf};

pub enum ImportScan {
    Added(PathBuf),
    Removed(PathBuf),
    Modified(PathBuf),
    Error(ImportError),
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

    pub fn error(error: impl Into<ImportError>) -> Self {
        Self::Error(error.into())
    }

    pub fn path(&self) -> &Path {
        match self {
            ImportScan::Added(path) => path,
            ImportScan::Removed(path) => path,
            ImportScan::Modified(path) => path,
            ImportScan::Error(error) => error.path(),
        }
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Folder {
    children: HashSet<PathBuf>,
}

impl Folder {
    pub fn add_child(&mut self, child: PathBuf) {
        self.children.insert(child);
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.children.contains(path)
    }

    pub fn replace(&mut self, other: Folder) -> Vec<PathBuf> {
        let mut removed = vec![];
        for child in self.children.drain() {
            if !other.contains(&child) {
                removed.push(child);
            }
        }

        self.children = other.children;

        removed
    }

    pub fn difference<'a>(
        &'a self,
        other: &'a Folder,
    ) -> Difference<'a, PathBuf, DefaultHashBuilder> {
        self.children.difference(&other.children)
    }
}

impl Asset for Folder {}
impl Settings for Folder {}

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
        database: &'a AssetDatabase,
    ) -> Vec<ImportScan> {
        let mut reader = source.reader(path);
        let paths = match reader.read_directory().await {
            Ok(paths) => paths,
            Err(error) => return vec![ImportScan::error((path, error))],
        };

        let mut settings = Folder::default();
        let mut imports = Vec::new();
        for path in paths {
            if path.is_dir() {
                imports.extend(Box::pin(Self::scan_folder(&path, source, database)).await);
            } else {
                imports.extend(Self::scan_file(&path, source, database).await);
            }

            settings.add_child(path);
        }

        let settings = if let Ok(mut prev_settings) = source.load_settings::<Folder>(path).await {
            imports.extend(
                prev_settings
                    .replace(settings)
                    .drain(..)
                    .map(|p| ImportScan::Removed(p)),
            );

            prev_settings
        } else {
            AssetSettings::new(AssetId::new::<Folder>(), settings)
        };

        if let Err(error) = source.save_settings(path, &settings).await {
            imports.push(ImportScan::error((path, error)));
        }

        imports
    }

    async fn scan_file<'a>(
        path: &'a Path,
        source: &'a AssetSource,
        database: &'a AssetDatabase,
    ) -> Option<ImportScan> {
        let library = database.library();
        let assets = match library.get_sources(source.id()) {
            Some(assets) => assets,
            None => return Some(ImportScan::Added(path.to_path_buf())),
        };

        let id = match assets.path_id(path) {
            Some(id) => id,
            None => return Some(ImportScan::Added(path.to_path_buf())),
        };

        let source_asset = match assets.get(id) {
            Some(asset) => asset,
            None => return Some(ImportScan::Added(path.to_path_buf())),
        };

        let mut reader = source.reader(&source.settings_path(path));
        match reader.exists().await {
            Ok(false) => return Some(ImportScan::Added(path.to_path_buf())),
            Err(_) => return None,
            _ => (),
        }

        let settings = match reader.read_to_end().await {
            Ok(bytes) => bytes,
            Err(error) => return Some(ImportScan::error((path, error))),
        };

        let mut reader = source.reader(&path);
        let asset = match reader.read_to_end().await {
            Ok(bytes) => bytes,
            Err(error) => return Some(ImportScan::error((path, error))),
        };

        let checksum = AssetSource::checksum(asset, settings);

        if checksum != source_asset.checksum {
            return Some(ImportScan::modified(path));
        }

        None
    }
}

impl WorldAction for ImportFolder {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportFolder {
    fn execute(&mut self, database: &super::AssetDatabase, _: &ecs::world::action::WorldActions) {
        if let Some(source) = database.config().source(&self.source) {
            let mut imports = vec![];
            for scan in block_on(Self::scan_folder(&self.path, source, database)) {
                match scan {
                    ImportScan::Added(path) | ImportScan::Modified(path) => imports.push(path),
                    _ => (),
                }
            }

            database
                .events()
                .push_front(ImportAssets::new(*source.id(), imports));
        }
    }
}

pub struct ImportAssets {
    source: SourceId,
    paths: Vec<PathBuf>,
}

impl ImportAssets {
    pub fn new(source: impl Into<SourceId>, paths: Vec<PathBuf>) -> Self {
        Self {
            source: source.into(),
            paths,
        }
    }

    async fn import_and_save<'a>(
        path: &Path,
        source: &AssetSource,
        database: &AssetDatabase,
    ) -> Result<(), ImportError> {
        let registry = database.config().registry();
        let cache = database.config().cache();
        let ext = match path.ext() {
            Some(ext) => ext,
            None => return Err(ImportError::missing_extension(path)),
        };

        let metadata = match registry.get_by_ext(ext) {
            Some(metadata) => metadata,
            None => return Err(ImportError::unsupported_extension(path)),
        };

        let import = metadata.import(path, source).await?;
        let (artifact, prev_meta) = metadata
            .save(import.main.asset, import.main.meta, cache)
            .map_err(|e| ImportError::from(path, e))
            .await?;

        let mut library = database.library_mut();
        let sources = library.get_sources_mut(source.id());
        let source = SourceAsset::new(import.checksum, path.to_path_buf(), AssetKind::Main);
        sources.add(artifact.meta.id, source);

        let mut futures = vec![];
        for sub_asset in import.sub {
            let metadata = match registry.get(&sub_asset.meta.id.ty()) {
                Some(metadata) => metadata,
                None => continue,
            };

            futures.push(metadata.save(sub_asset.asset, sub_asset.meta, cache));
        }

        let mut errors = vec![];
        for artifact in join_all(futures).await {
            match artifact {
                Ok((artifact, _)) => {
                    let source =
                        SourceAsset::new(import.checksum, path.to_path_buf(), AssetKind::Sub);
                    sources.add(artifact.meta.id, source);
                }
                Err(error) => errors.push(error),
            }
        }

        todo!()
    }
}

impl WorldAction for ImportAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&mut self, database: &AssetDatabase, actions: &WorldActions) {
        if let Some(source) = database.config().source(&self.source) {
            const POOL_SIZE: usize = 100;
            let mut pool = AsyncTaskPool::new();
            for paths in self.paths.chunks(POOL_SIZE) {
                for path in paths {
                    pool.spawn(Self::import_and_save(path, source, database));
                }
            }
        }

        todo!()
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

impl WorldAction for ImportAsset {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(ImportAssets::new(
            self.source,
            vec![self.path],
        ))))
    }
}
