use super::{AssetDatabase, AssetEvent, StartAssetEvent};
use crate::{
    asset::{Asset, AssetId, AssetSettings, Settings},
    cache::AssetCache,
    importer::registry::ImportError,
    io::{AssetIoError, AssetSource, PathExt, SourceId},
    library::{ArtifactMeta, SourceAsset},
};
use ecs::{
    event::{Event, Events},
    task::AsyncTaskPool,
    world::action::{WorldAction, WorldActions},
};
use futures::{executor::block_on, future::join_all, TryFutureExt};
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
        let source_checksum = {
            let library = database.library();
            let assets = match library.get_sources(source.id()) {
                Some(assets) => assets,
                None => return Some(ImportScan::Added(path.to_path_buf())),
            };

            let source_asset = match assets.get(path) {
                Some(asset) => asset,
                None => return Some(ImportScan::Added(path.to_path_buf())),
            };

            source_asset.checksum
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

        if checksum != source_checksum {
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
    ) -> Result<(ArtifactMeta, Vec<AssetId>), ImportError> {
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
        let source = SourceAsset::new(import.checksum, artifact.meta.id);
        sources.add_main(path.to_path_buf(), source);

        let mut futures = vec![];
        for sub_asset in import.sub {
            let metadata = match registry.get(&sub_asset.meta.id.ty()) {
                Some(metadata) => metadata,
                None => continue,
            };

            futures.push(metadata.save(sub_asset.asset, sub_asset.meta, cache));
        }

        for artifact in join_all(futures).await {
            match artifact {
                Ok((artifact, _)) => sources.add_sub(artifact.meta.id, path.to_path_buf()),
                Err(error) => return Err(ImportError::from(path, error)),
            }
        }

        let mut removed = vec![];
        if let Some(meta) = prev_meta {
            for dep in meta.dependencies {
                if !artifact.meta.dependencies.contains(&dep) {
                    removed.push(dep);
                }
            }

            for sub in meta.sub_assets {
                if !artifact.meta.sub_assets.contains(&sub) {
                    removed.push(sub);
                }
            }
        }

        Ok((artifact.meta, removed))
    }
}

impl WorldAction for ImportAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for ImportAssets {
    fn execute(&mut self, database: &AssetDatabase, _: &WorldActions) {
        if let Some(source) = database.config().source(&self.source) {
            const POOL_SIZE: usize = 100;
            let mut pool = AsyncTaskPool::new();
            let mut removed_assets = vec![];
            let mut reload_assets = vec![];
            let mut errors = vec![];
            for paths in self.paths.chunks(POOL_SIZE) {
                for path in paths {
                    pool.spawn(Self::import_and_save(path, source, database));
                }

                for result in block_on(pool.run()) {
                    match result {
                        Ok((meta, removed)) => {
                            reload_assets.push(meta.id);
                            reload_assets.extend(meta.sub_assets);
                            reload_assets.extend(meta.dependencies);
                            removed_assets.extend(removed);
                        }
                        Err(error) => errors.push(error),
                    }
                }
            }

            database
                .events()
                .push_front(RemoveAssets::new(self.source, removed_assets));
            database
                .events()
                .push_front(ImportErrors::new(self.source, errors));
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

impl WorldAction for ImportAsset {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(ImportAssets::new(
            self.source,
            vec![self.path],
        ))))
    }
}

#[derive(Debug)]
pub struct RemoveAssetError {
    id: AssetId,
    error: AssetIoError,
}

impl RemoveAssetError {
    pub fn new(id: AssetId, error: AssetIoError) -> Self {
        Self { id, error }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn error(&self) -> &AssetIoError {
        &self.error
    }
}

impl std::fmt::Display for RemoveAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to remove asset: {:?}", self.id)
    }
}
impl std::error::Error for RemoveAssetError {}
impl Event for RemoveAssetError {}
impl WorldAction for RemoveAssetError {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<RemoveAssetError>>();
        events.add(self);
        Some(())
    }
}

pub struct RemoveAssets {
    source: SourceId,
    ids: Vec<AssetId>,
}

impl RemoveAssets {
    pub fn new(source: impl Into<SourceId>, ids: Vec<AssetId>) -> Self {
        Self {
            source: source.into(),
            ids,
        }
    }

    async fn remove_asset<'a>(
        &'a self,
        id: &'a AssetId,
        database: &'a AssetDatabase,
        cache: &'a AssetCache,
    ) -> Result<Vec<AssetId>, RemoveAssetError> {
        let is_sub = {
            let mut library = database.library_mut();
            let sources = library.get_sources_mut(&self.source);
            sources.remove(id).is_none()
        };

        let artifact = cache
            .remove_artifact(*id)
            .await
            .map_err(|e| RemoveAssetError::new(*id, e))?;

        if is_sub {
            return Ok(vec![*id]);
        }

        let futures = {
            let mut futures = vec![];
            let mut library = database.library_mut();
            let sources = library.get_sources_mut(&self.source);
            for id in artifact.meta.sub_assets {
                sources.remove_sub(&id);
                futures.push(cache.remove_artifact(id));
            }

            futures
        };
        let mut unloads = vec![*id];
        for result in join_all(futures).await {
            if let Ok(artifact) = result {
                unloads.push(artifact.meta.id);
            }
        }

        unloads.push(artifact.meta.id);

        Ok(unloads)
    }
}

impl WorldAction for RemoveAssets {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(StartAssetEvent::new(self)))
    }
}

impl AssetEvent for RemoveAssets {
    fn execute(&mut self, database: &AssetDatabase, actions: &ecs::world::action::WorldActions) {
        let cache = database.config().cache();
        const POOL_SIZE: usize = 100;
        let mut pool = AsyncTaskPool::new();
        let mut unload_assets = vec![];
        let mut errors = vec![];
        for ids in self.ids.chunks(POOL_SIZE) {
            for id in ids {
                pool.spawn(self.remove_asset(id, database, cache));
            }

            for result in block_on(pool.run()) {
                match result {
                    Ok(ids) => unload_assets.extend(ids),
                    Err(error) => errors.push(error),
                }
            }
        }

        actions.extend(errors);
    }
}

pub struct ImportErrors {
    source: SourceId,
    errors: Vec<ImportError>,
}

impl ImportErrors {
    pub fn new(source: impl Into<SourceId>, errors: Vec<ImportError>) -> Self {
        Self {
            source: source.into(),
            errors,
        }
    }

    pub fn source(&self) -> &SourceId {
        &self.source
    }

    pub fn errors(&self) -> &[ImportError] {
        &self.errors
    }
}

impl WorldAction for ImportErrors {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<StartAssetEvent>>();
        Some(events.add(self.into()))
    }
}

impl AssetEvent for ImportErrors {
    fn execute(&mut self, database: &AssetDatabase, actions: &ecs::world::action::WorldActions) {
        let mut library = database.library_mut();
        let sources = library.get_sources_mut(&self.source);
        let mut remove = vec![];
        for error in &self.errors {
            let source = match sources.get(error.path()) {
                Some(source) => source,
                None => continue,
            };

            remove.push(source.id);
        }

        let notify = NotifyImportErrors::new(std::mem::take(&mut self.errors));
        database
            .events()
            .push_front(RemoveAssets::new(self.source, remove));
        actions.add(notify);
    }
}

pub struct NotifyImportErrors {
    errors: Vec<ImportError>,
}

impl NotifyImportErrors {
    pub fn new(errors: Vec<ImportError>) -> Self {
        Self { errors }
    }

    pub fn errors(&self) -> &[ImportError] {
        &self.errors
    }
}

impl WorldAction for NotifyImportErrors {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        let events = world.resource_mut::<Events<ImportError>>();
        Some(events.extend(self.errors))
    }
}
