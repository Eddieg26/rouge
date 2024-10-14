use super::{AssetEvent, StartAssetEvent};
use crate::{
    asset::{Asset, AssetId, AssetSettings, Settings},
    cache::AssetCache,
    database::{registry::AssetRegistry, AssetDatabase},
    importer::ImportError,
    io::{AssetIoError, AssetSource, PathExt, SourceId},
};
use ecs::{
    event::{Event, Events},
    task::AsyncTaskPool,
    world::action::{BulkEvents, WorldAction},
};
use futures::executor::block_on;
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
            match reader.is_directory(&path).await.unwrap_or(false) {
                true => imports.extend(Box::pin(Self::scan_folder(&path, source, database)).await),
                false => imports.extend(Self::scan_file(&path, source, database).await),
            }

            settings.add_child(path);
        }

        let settings = if let Ok(mut prev_settings) = source.load_settings::<Folder>(path).await {
            for removed in prev_settings.replace(settings) {
                imports.push(ImportScan::Removed(removed));
            }

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
        let id = {
            let library = database.library();
            let assets = match library.get_sources(source.id()) {
                Some(assets) => assets,
                None => return Some(ImportScan::Added(path.to_path_buf())),
            };

            match assets.get(path) {
                Some(id) => id,
                None => return Some(ImportScan::Added(path.to_path_buf())),
            }
        };

        let header = match database.config().cache().load_artifact_header(id).await {
            Ok(header) => header,
            Err(_) => return Some(ImportScan::Added(path.to_path_buf())),
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

        if AssetSource::checksum(asset, settings) != header.checksum {
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
    pub fn new(source: SourceId, paths: Vec<PathBuf>) -> Self {
        Self { source, paths }
    }

    pub async fn import_and_save<'a>(
        path: &'a Path,
        registry: &'a AssetRegistry,
        source: &'a AssetSource,
        cache: &'a AssetCache,
    ) -> Result<(Vec<AssetId>, Vec<AssetId>), ImportError> {
        let ext = match path.ext() {
            Some(ext) => ext,
            None => return Err(ImportError::missing_extension(path)),
        };

        let metadata = match registry.get_by_ext(ext) {
            Some(metadata) => metadata,
            None => return Err(ImportError::unsupported_extension(path)),
        };

        let artifacts = metadata.import(path, source).await?;
        let prev_meta = cache.load_artifact_meta(&artifacts.main.meta.id).await.ok();

        if let Err(e) = cache.save_artifact(&artifacts.main).await {
            return Err(ImportError::from(path, e));
        }

        for artifact in artifacts.sub {
            if let Err(e) = cache.save_artifact(&artifact).await {
                return Err(ImportError::from(path, e));
            }
        }

        let mut removed = vec![];
        let mut reload = vec![artifacts.main.meta.id];
        reload.extend(artifacts.main.meta.dependencies);
        if let Some(prev) = prev_meta {
            for sub in prev.sub_assets {
                if !artifacts.main.meta.sub_assets.contains(&sub) {
                    removed.push(sub);
                }
            }

            reload.extend(prev.dependencies);
        }

        Ok((reload, removed))
    }
}

impl WorldAction for ImportAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&mut self, database: &AssetDatabase, _: &ecs::world::action::WorldActions) {
        if let Some(source) = database.config().source(&self.source) {
            let cache = database.config().cache();
            let registry = database.config().registry();
            let mut pool = AsyncTaskPool::new();
            let mut reload = vec![];
            let mut remove = vec![];
            let mut errors = vec![];

            for paths in self.paths.chunks(100) {
                for path in paths {
                    pool.spawn(Self::import_and_save(path, registry, source, cache));
                }

                for result in block_on(pool.run()) {
                    match result {
                        Ok((reloaded, removed)) => {
                            reload.extend(reloaded);
                            remove.extend(removed);
                        }
                        Err(error) => errors.push(error),
                    }
                }
            }

            let mut events = database.events();
            if !errors.is_empty() {
                events.push_front(ImportErrors::new(self.source, errors));
            }

            if !remove.is_empty() {
                events.push_front(RemoveAssets::new(self.source, remove));
            }

            // TODO: Reload assets
        }
    }
}

pub struct ImportErrors {
    source: SourceId,
    errors: Vec<ImportError>,
}

impl ImportErrors {
    pub fn new(source: SourceId, errors: Vec<ImportError>) -> Self {
        Self { source, errors }
    }
}

impl WorldAction for ImportErrors {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportErrors {
    fn execute(&mut self, database: &AssetDatabase, actions: &ecs::world::action::WorldActions) {
        let library = database.library_mut();
        if let Some(source) = library.get_sources(&self.source) {
            let mut remove = vec![];
            for error in &self.errors {
                if let Some(id) = source.get(error.path()) {
                    remove.push(id);
                }
            }

            database
                .events()
                .push_front(RemoveAssets::new(self.source, remove));
        }

        actions.add(BulkEvents::new(std::mem::take(&mut self.errors)));
    }
}

#[derive(Debug)]
pub struct RemoveError {
    id: AssetId,
    error: AssetIoError,
}

impl RemoveError {
    pub fn new(id: AssetId, error: AssetIoError) -> Self {
        Self { id, error }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn error(&self) -> &AssetIoError {
        &self.error
    }
}

impl std::fmt::Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to remove asset: {:?}", self.id)
    }
}

impl Event for RemoveError {}

pub struct RemoveAssets {
    source: SourceId,
    assets: Vec<AssetId>,
}

impl RemoveAssets {
    pub fn new(source: SourceId, assets: Vec<AssetId>) -> Self {
        Self { source, assets }
    }

    pub async fn remove<'a>(
        &'a self,
        id: &'a AssetId,
        database: &'a AssetDatabase,
    ) -> (Vec<AssetId>, Vec<RemoveError>) {
        let mut removed = vec![];
        let mut errors = vec![];
        let cache = database.config().cache();
        let artifact = match cache.remove_artifact(*id).await {
            Ok(artifact) => artifact,
            Err(error) => {
                errors.push(RemoveError::new(*id, error));
                return (vec![*id], errors);
            }
        };

        let mut library = database.library_mut();
        let sources = library.get_sources_mut(&self.source);
        sources.remove(&artifact.meta.path);
        std::mem::drop(library);

        removed.push(artifact.meta.id);
        for sub in artifact.meta.sub_assets {
            removed.push(sub);
            if let Err(error) = cache.remove_artifact(sub).await {
                errors.push(RemoveError::new(sub, error));
            }
        }

        (removed, errors)
    }
}

impl AssetEvent for RemoveAssets {
    fn execute(&mut self, database: &AssetDatabase, actions: &ecs::world::action::WorldActions) {
        let mut pool = AsyncTaskPool::new();
        let mut unload = vec![];
        let mut errors = vec![];
        for ids in self.assets.chunks(100) {
            for id in ids {
                pool.spawn(self.remove(id, database));
            }

            for (removed, errs) in block_on(pool.run()) {
                unload.extend(removed);
                errors.extend(errs);
            }
        }

        actions.add(BulkEvents::new(errors));

        // TODO: Unload assets
    }
}
