use super::AssetAction;
use crate::{
    asset::{Asset, AssetId, AssetMetadata, Settings},
    database::config::AssetConfig,
    importer::ImportError,
    io::{
        cache::{Artifact, AssetCache, ProcessedInfo, SourceMap},
        source::{AssetPath, AssetSource, AssetSourceName},
        AssetIoError, PathExt,
    },
};
use async_std::sync::RwLock;
use ecs::{
    core::IndexSet,
    task::AsyncTaskPool,
    world::action::{BatchEvents, WorldAction, WorldActions},
};
use futures::{executor::block_on, StreamExt, TryFutureExt};
use hashbrown::{hash_set::Difference, DefaultHashBuilder, HashMap, HashSet};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub enum ImportScan {
    Added(AssetPath),
    Removed(AssetPath),
    Modified(AssetPath),
    Error(ImportError),
}

impl ImportScan {
    pub fn path(&self) -> Option<&Path> {
        match self {
            ImportScan::Added(path) => Some(path.path()),
            ImportScan::Removed(path) => Some(path.path()),
            ImportScan::Modified(path) => Some(path.path()),
            ImportScan::Error(error) => error.path().map(|path| path.path()),
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
    source: AssetSourceName,
    path: PathBuf,
}

impl ImportFolder {
    pub fn new(path: PathBuf) -> Self {
        Self {
            source: AssetSourceName::Default,
            path,
        }
    }

    pub fn from_source(source: impl Into<AssetSourceName>, path: PathBuf) -> Self {
        Self {
            source: source.into(),
            path,
        }
    }

    async fn scan_folder<'a>(
        asset_path: AssetPath,
        map: &'a Arc<RwLock<SourceMap>>,
        source: &'a AssetSource,
        config: &'a AssetConfig,
    ) -> Vec<ImportScan> {
        let path = asset_path.path();
        let mut paths = match source.read_dir(path).await {
            Ok(paths) => paths,
            Err(error) => return vec![ImportScan::Error(ImportError::import(path, error))],
        };

        let mut metadata = Folder::default();
        let mut imports = Vec::new();
        while let Some(path) = paths.next().await {
            let asset_path = AssetPath::new(asset_path.source().clone(), path.to_path_buf());
            match source.is_dir(asset_path.path()).await.unwrap_or(false) {
                true => imports
                    .extend(Box::pin(Self::scan_folder(asset_path, map, source, config)).await),
                false => imports.extend(Self::scan_file(asset_path, map, source, config).await),
            }

            metadata.add_child(path);
        }

        let metadata =
            if let Ok(mut prev_metadata) = source.load_metadata::<Folder, Folder>(path).await {
                for removed in prev_metadata.replace(metadata) {
                    let path = AssetPath::new(asset_path.source().clone(), removed);
                    imports.push(ImportScan::Removed(path));
                }

                prev_metadata
            } else {
                AssetMetadata::new(AssetId::new::<Folder>(), metadata)
            };

        if let Err(error) = source.save_metadata(path, &metadata).await {
            imports.push(ImportScan::Error(ImportError::import(path, error)));
        }

        imports
    }

    async fn scan_file<'a>(
        asset_path: AssetPath,
        map: &'a Arc<RwLock<SourceMap>>,
        source: &'a AssetSource,
        config: &'a AssetConfig,
    ) -> Option<ImportScan> {
        let path = asset_path.path();
        if let Some("meta") = path.ext() {
            return None;
        }

        let id = {
            let map = map.read().await;
            match map.get(&asset_path) {
                Some(id) => id,
                None => return Some(ImportScan::Added(asset_path)),
            }
        };

        match source.exists(&AssetSource::metadata_path(path)).await {
            Ok(false) => return Some(ImportScan::Added(asset_path)),
            Err(_) => return Some(ImportScan::Added(asset_path)),
            _ => (),
        }

        let meta = match config.cache().load_artifact_meta(&id).await {
            Ok(meta) => meta,
            Err(_) => return Some(ImportScan::Added(asset_path)),
        };

        let settings = match source.read_metadata_bytes(path).await {
            Ok(bytes) => bytes,
            Err(error) => return Some(ImportScan::Error(ImportError::import(path, error))),
        };

        let asset = match source.read_asset_bytes(path).await {
            Ok(bytes) => bytes,
            Err(error) => return Some(ImportScan::Error(ImportError::import(path, error))),
        };

        let checksum = ProcessedInfo::checksum(&asset, &settings);

        match meta.processed.map(|p| p.checksum == checksum) {
            Some(true) => None,
            _ => Some(ImportScan::Modified(asset_path)),
        }
    }

    pub async fn import(
        path: AssetPath,
        config: &AssetConfig,
        source: &AssetSource,
        state: &ImportState,
    ) -> Vec<ImportScan> {
        Self::scan_folder(path, &state.source_map, source, config).await
    }
}

impl WorldAction for ImportFolder {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        // let events = world.resource_mut::<Events<StartAssetEvent>>();
        // Some(events.add(StartAssetEvent::new(self)))
        Some(())
    }
}

impl AssetAction for ImportFolder {
    fn execute(&mut self, config: &AssetConfig, actions: &WorldActions) {
        if let Some(source) = config.source(&self.source) {
            let mut state = ImportState::new(config);
            let path = AssetPath::new(self.source.clone(), self.path.clone());
            let scans = block_on(ImportFolder::import(path, config, source, &state));

            let mut imports = vec![];
            let mut removed = vec![];
            let mut errors = vec![];

            for scan in scans {
                match scan {
                    ImportScan::Added(path) => imports.push(path),
                    ImportScan::Modified(path) => imports.push(path),
                    ImportScan::Removed(path) => removed.push(path),
                    ImportScan::Error(error) => errors.push(error),
                }
            }

            let import = async {
                ImportAssets::import(config, &imports, &mut state).await;
                ImportAssets::process(config, &mut state).await;
                RemoveArtifacts::remove(config, &mut state).await;
            };

            block_on(import);
            actions.add(BatchEvents::new(errors));

            // TODO: Unload Removed assets
            // TODO: Hot reload assets
        }
    }
}

#[derive(Debug, Default)]
pub struct ImportResult {
    pub imported: Vec<AssetId>,
    pub removed: Vec<AssetId>,
    pub dependencies: HashMap<AssetId, Vec<AssetId>>,
}

impl ImportResult {
    pub fn new(
        imported: Vec<AssetId>,
        removed: Vec<AssetId>,
        dependencies: HashMap<AssetId, Vec<AssetId>>,
    ) -> Self {
        Self {
            imported,
            removed,
            dependencies,
        }
    }
}

#[derive(Debug, Default)]
pub struct ImportState {
    pub imported: IndexSet<AssetId>,
    pub removed: Vec<AssetId>,
    pub errors: Vec<ImportError>,
    pub source_map: Arc<RwLock<SourceMap>>,
    pub dependencies: HashMap<AssetId, Vec<AssetId>>,
}

impl ImportState {
    pub fn new(config: &AssetConfig) -> Self {
        Self {
            source_map: block_on(Self::load_source_map(config.cache())),
            ..Default::default()
        }
    }

    pub async fn error_ids(&self) -> Vec<AssetId> {
        let map = self.source_map.read().await;
        self.errors
            .iter()
            .filter_map(|error| {
                let path = error.path()?;
                map.get(path)
            })
            .collect()
    }

    async fn load_source_map<'a>(cache: &'a AssetCache) -> Arc<RwLock<SourceMap>> {
        let source_map = match cache.load_source_map().await {
            Ok(source_map) => source_map,
            Err(_) => SourceMap::new(),
        };

        Arc::new(RwLock::new(source_map))
    }
}

pub struct ImportAssets {
    paths: Vec<AssetPath>,
}

impl ImportAssets {
    pub fn new(paths: impl IntoIterator<Item = impl Into<AssetPath>>) -> Self {
        Self {
            paths: paths.into_iter().map(Into::into).collect(),
        }
    }

    pub fn paths(&self) -> &[AssetPath] {
        &self.paths
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetPath> {
        self.paths.iter()
    }

    pub async fn import(config: &AssetConfig, paths: &[AssetPath], state: &mut ImportState) {
        let mut pool = AsyncTaskPool::new();
        for paths in paths.chunks(100) {
            for path in paths {
                pool.spawn(Self::import_asset(&state.source_map, config, path));
            }

            for result in block_on(pool.run()) {
                match result {
                    Ok(result) => {
                        for (id, deps) in result.dependencies {
                            state.dependencies.insert(id, deps);
                        }

                        state.imported.extend(result.imported);
                        state.removed.extend(result.removed);
                    }
                    Err(error) => state.errors.push(error),
                }
            }
        }
    }

    pub async fn process(config: &AssetConfig, state: &mut ImportState) {
        let mut order = vec![];
        while !state.dependencies.is_empty() {
            let mut group = vec![];
            for (id, deps) in state.dependencies.iter() {
                if deps.iter().all(|dep| !state.dependencies.contains_key(dep)) {
                    group.push(*id);
                }
            }

            if group.is_empty() {
                panic!("Cyclic dependency detected");
            }

            state.dependencies.retain(|id, _| !group.contains(id));
            order.extend(group);
        }

        let mut pool = AsyncTaskPool::new();
        for ids in order.chunks(100) {
            for id in ids {
                pool.spawn(Self::process_asset(&state.source_map, config, id));
            }

            for result in block_on(pool.run()) {
                match result {
                    Ok(result) => state.removed.extend(result),
                    Err(error) => state.errors.push(error),
                };
            }
        }
    }

    pub async fn import_asset<'a>(
        map: &'a Arc<RwLock<SourceMap>>,
        config: &'a AssetConfig,
        asset_path: &'a AssetPath,
    ) -> Result<ImportResult, ImportError> {
        let source = match config.source(asset_path.source()) {
            Some(source) => source,
            None => return Ok(ImportResult::default()),
        };

        let path = asset_path.path();
        let ext = match path.ext() {
            Some("meta") => return Ok(ImportResult::default()),
            Some(ext) => ext,
            None => return Ok(ImportResult::default()),
        };

        let meta = match config.registry().get_by_ext(ext) {
            Some(importer) => importer,
            None => return Ok(ImportResult::default()),
        };

        let artifacts = meta.import(asset_path, source).await?;
        let mut removed_assets = vec![];
        let mut imported = vec![];
        let mut dependencies = HashMap::new();

        for artifact in artifacts {
            imported.push(artifact.meta.id);
            match config.registry().has_processor(artifact.meta.id.ty()) {
                true => {
                    let removed = Self::save_temp_artifact(config.cache(), &artifact)
                        .await
                        .map_err(|e| ImportError::import(path, e))?;
                    removed_assets.extend(removed);
                    dependencies.insert(artifact.meta.id, artifact.meta.dependencies);
                }
                false => {
                    let removed = Self::save_artifact(map, source, config.cache(), artifact)
                        .await
                        .map_err(|e| ImportError::import(path, e))?;
                    removed_assets.extend(removed);
                }
            }
        }

        Ok(ImportResult::new(imported, removed_assets, dependencies))
    }

    pub async fn save_temp_artifact<'a>(
        cache: &'a AssetCache,
        artifact: &'a Artifact,
    ) -> Result<Vec<AssetId>, crate::io::AssetIoError> {
        let temp_path = cache.temp_artifact_path(&artifact.meta.id);
        cache.save_artifact(&temp_path, artifact).await?;
        Ok(vec![])
    }

    pub async fn save_artifact<'a>(
        map: &'a Arc<RwLock<SourceMap>>,
        source: &'a AssetSource,
        cache: &'a AssetCache,
        mut artifact: Artifact,
    ) -> Result<Vec<AssetId>, AssetIoError> {
        let path = cache.artifact_path(&artifact.meta.id);
        let prev_meta = cache.load_artifact_meta(&artifact.meta.id).await.ok();

        let mut removed = vec![];
        if let Some(prev) = prev_meta {
            for sub in prev.children {
                if !artifact.meta.children.contains(&sub) {
                    removed.push(sub);
                }
            }
        }

        let mut processed_info = {
            let asset = source.read_asset_bytes(artifact.meta.path.path()).await?;
            let metadata = source
                .read_metadata_bytes(artifact.meta.path.path())
                .await?;

            let checksum = ProcessedInfo::checksum(&asset, &metadata);
            ProcessedInfo::new(checksum)
        };

        for dep in &artifact.meta.dependencies {
            let meta = cache.load_artifact_meta(dep).await;
            if let Some(info) = meta.ok().and_then(|m| m.processed) {
                processed_info.add_dependency(*dep, info.checksum);
            }
        }

        artifact.meta.processed = Some(processed_info);

        cache.save_artifact(&path, &artifact).await?;
        map.write()
            .await
            .insert(artifact.meta.path.clone(), artifact.meta.id);

        Ok(removed)
    }

    pub async fn process_asset<'a>(
        map: &'a Arc<RwLock<SourceMap>>,
        config: &'a AssetConfig,
        id: &'a AssetId,
    ) -> Result<Vec<AssetId>, ImportError> {
        let meta = match config.registry().get(id.ty()) {
            Some(meta) => meta,
            None => return Ok(vec![]),
        };

        let temp_path = config.cache().temp_artifact_path(id);
        let artifact = config
            .cache()
            .load_artifact(&temp_path)
            .await
            .map_err(|e| ImportError::proccess(*id, None, e))?;

        let _ = config.cache().remove_artifact(&temp_path).await;

        let source = match config.source(artifact.meta.path.source()) {
            Some(source) => source,
            None => return Ok(vec![]),
        };

        let artifact = match meta.process(id, source, config.cache()) {
            Some(result) => result
                .await
                .map_err(|e| ImportError::proccess(*id, Some(artifact.meta.path), e))?,
            None => artifact,
        };

        let path = artifact.meta.path.clone();
        Self::save_artifact(map, source, config.cache(), artifact)
            .map_err(|e| ImportError::proccess(*id, Some(path), e))
            .await
    }
}

impl AssetAction for ImportAssets {
    fn execute(&mut self, config: &AssetConfig, _actions: &WorldActions) {
        let mut state = ImportState::new(config);
        let import = async {
            ImportAssets::import(config, &self.paths, &mut state).await;
            ImportAssets::process(config, &mut state).await;
            RemoveArtifacts::remove(config, &mut state).await;

            let paths = RefreshAssets::refresh(config, &state.source_map).await;

            ImportAssets::import(config, &paths, &mut state).await;
            ImportAssets::process(config, &mut state).await;
            RemoveArtifacts::remove(config, &mut state).await;

            // Unload Removed assets
            // Hot reload assets
        };

        block_on(import);
    }
}

pub struct RemoveArtifacts {
    ids: Vec<AssetId>,
}

impl RemoveArtifacts {
    pub fn new(ids: impl Iterator<Item = impl Into<AssetId>>) -> Self {
        Self {
            ids: ids.map(Into::into).collect(),
        }
    }

    pub fn ids(&self) -> &[AssetId] {
        &self.ids
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetId> {
        self.ids.iter()
    }

    pub async fn remove(config: &AssetConfig, state: &mut ImportState) {
        let (removed, errors) = {
            let mut pool = AsyncTaskPool::new();
            let mut removed = vec![];
            let mut errors = vec![];

            state.removed.extend(state.error_ids().await);
            for ids in state.removed.chunks(100) {
                for id in ids {
                    pool.spawn(Self::remove_artifact(&state.source_map, config, id));
                }

                for result in block_on(pool.run()) {
                    match result {
                        Ok(result) => removed.extend(result),
                        Err(error) => errors.push(error),
                    }
                }
            }

            (removed, errors)
        };

        state.removed.extend(removed);
        state.errors.extend(errors);
    }

    pub async fn remove_artifact(
        map: &Arc<RwLock<SourceMap>>,
        config: &AssetConfig,
        id: &AssetId,
    ) -> Result<Vec<AssetId>, ImportError> {
        let meta = match config.cache().load_artifact_meta(&id).await {
            Ok(meta) => meta,
            Err(_) => return Ok(vec![]),
        };

        let mut removed = vec![];
        for child in meta.children {
            let child_path = config.cache().artifact_path(&child);
            let _ = config.cache().remove_artifact(&child_path).await;
            removed.push(child);
        }

        let path = config.cache().artifact_path(&id);
        let _ = config.cache().remove_artifact(&path).await;

        let mut map = map.write().await;
        map.remove(&meta.path);

        Ok(removed)
    }
}

pub struct RefreshAssets;

impl RefreshAssets {
    pub async fn has_dependencies_changed(id: AssetId, cache: &AssetCache) -> bool {
        let meta = match cache.load_artifact_meta(&id).await {
            Ok(meta) => meta,
            Err(_) => return true,
        };

        let info = match meta.processed {
            Some(info) => info,
            None => return false,
        };

        for dep in info.dependencies {
            let dep_meta = match cache.load_artifact_meta(&dep.id).await {
                Ok(meta) => meta,
                Err(_) => return true,
            };

            match dep_meta.processed {
                Some(dep_info) => {
                    if dep_info.checksum != dep.checksum {
                        return true;
                    } else if Box::pin(Self::has_dependencies_changed(dep.id, cache)).await {
                        return true;
                    }
                }
                None => continue,
            }
        }

        return false;
    }

    pub async fn refresh_file(
        asset_path: AssetPath,
        cache: &AssetCache,
        map: &Arc<RwLock<SourceMap>>,
    ) -> Option<AssetPath> {
        if let Some("meta") = asset_path.path().ext() {
            return None;
        }

        let id = match map.read().await.get(&asset_path) {
            Some(id) => id,
            None => return Some(asset_path),
        };

        match Self::has_dependencies_changed(id, cache).await {
            true => Some(asset_path),
            false => None,
        }
    }

    pub async fn refresh_folder(
        asset_path: AssetPath,
        source: &AssetSource,
        cache: &AssetCache,
        map: &Arc<RwLock<SourceMap>>,
    ) -> Vec<AssetPath> {
        let mut paths = match source.read_dir("").await {
            Ok(paths) => paths,
            Err(_) => return vec![],
        };

        let mut imports = vec![];
        while let Some(path) = paths.next().await {
            let asset_path = AssetPath::new(asset_path.source().clone(), path.to_path_buf());
            match source.is_dir(asset_path.path()).await {
                Ok(true) => imports
                    .extend(Box::pin(Self::refresh_folder(asset_path, source, cache, map)).await),
                Ok(false) => imports.extend(Self::refresh_file(asset_path, cache, map).await),
                _ => continue,
            }
        }

        imports
    }

    pub async fn refresh(config: &AssetConfig, map: &Arc<RwLock<SourceMap>>) -> Vec<AssetPath> {
        let mut pool = AsyncTaskPool::new();
        for (name, source) in config.sources().iter() {
            let path = AssetPath::new(name.clone(), "");
            pool.spawn(Self::refresh_folder(path, source, config.cache(), map));
        }

        block_on(pool.run()).into_iter().flatten().collect()
    }
}
