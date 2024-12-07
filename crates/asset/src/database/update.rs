use super::{
    config::AssetConfig,
    events::{ReloadAssets, UnloadAssets},
    state::{LoadState, SharedStates},
    AssetDatabase, DatabaseEvent,
};
use crate::{
    asset::{Asset, AssetId, AssetMetadata, Settings},
    importer::{ImportError, LoadError},
    io::{
        cache::{ArtifactMeta, AssetCache, AssetInfo, ErasedLoadedAsset, LoadPath, SharedLibrary},
        source::{AssetPath, AssetSource},
    },
};
use ecs::{core::IndexSet, world::action::BatchEvents};
use futures::future::join_all;
use futures_lite::StreamExt;
use hashbrown::{HashMap, HashSet};
use std::{ops::Deref, path::PathBuf};

pub struct AssetImporter;

impl AssetImporter {
    pub async fn import(&self, mut paths: HashSet<AssetPath>, database: &AssetDatabase) {
        let config = &database.config;
        let library = &database.library;
        let states = &database.states;
        let actions = &database.actions;

        let mut imported = IndexSet::new();
        let mut reload = vec![];
        let mut unload = vec![];

        let _ = config.cache().create_unprocessed_dir().await;

        while !paths.is_empty() {
            let mut assets = HashMap::new();
            let mut errors = vec![];

            for path in paths.drain() {
                imported.insert(path.clone());
                match self.import_asset(&path, config, library).await {
                    Ok(imported) => {
                        for asset in imported {
                            assets.insert(asset.id, asset);
                        }
                    }
                    Err(error) => errors.push(error),
                }
            }

            for assets in self.get_process_order(assets) {
                let mut pool = vec![];
                for asset in assets {
                    pool.push(self.process_asset(asset, config, library));
                }

                for result in join_all(pool.drain(..)).await {
                    match result {
                        Ok(result) => {
                            for id in result.removed_dependencies {
                                states.write().await.remove_dependent(id, result.asset.id);
                            }

                            if let Some(meta) = config.registry().get(result.asset.id.ty()) {
                                actions.add(meta.imported(result.asset.path, result.asset.id));
                            }

                            reload.push(result.asset.id);
                            unload.extend(result.removed_assets);
                        }
                        Err(error) => errors.push(error),
                    }
                }
            }

            for error in &errors {
                if let Some(id) = import_error_id(error, library) {
                    AssetRefresher.remove_artifact(id, config.cache()).await;
                    unload.push(id);
                }
            }

            actions.add(BatchEvents::new(errors));
            let mut result = AssetRefresher
                .refresh_sources(config, library, RefreshMode::DEPS_MODIFIED)
                .await;

            paths.extend(result.imports.drain().filter_map(|path| {
                if imported.contains(&path) {
                    None
                } else {
                    Some(path)
                }
            }));
        }

        let _ = config
            .cache()
            .save_library(library.read().await.deref())
            .await;

        let _ = config.cache().remove_unprocessed_dir().await;

        actions.add(UnloadAssets::new(unload));
        actions.add(ReloadAssets::new(reload));
    }

    async fn import_asset(
        &self,
        path: &AssetPath,
        config: &AssetConfig,
        library: &SharedLibrary,
    ) -> Result<Vec<ImportedAsset>, ImportError> {
        let source = match config.source(path.source()) {
            Some(source) => source,
            None => return Err(ImportError::InvalidSource { path: path.clone() }),
        };

        let ext = match path.ext() {
            Some("meta") => return Ok(vec![]),
            Some(ext) => ext,
            None => return Err(ImportError::InvalidExtension { path: path.clone() }),
        };

        let meta = match config.registry().get_by_ext(ext) {
            Some(meta) => meta,
            None => return Err(ImportError::InvalidExtension { path: path.clone() }),
        };

        let mut assets = vec![];

        for artifact in meta.import(path, source).await? {
            let id = artifact.id();
            let temp_path = config.cache().unprocessed_artifact_path(&id);
            match config.cache().save_artifact(&temp_path, &artifact).await {
                Ok(_) => {
                    let asset =
                        ImportedAsset::new(id, artifact.path().clone(), artifact.meta.dependencies);
                    assets.push(asset);
                }
                Err(error) => return Err(ImportError::import(artifact.meta.path, error)),
            };

            let info = AssetInfo::new(id, artifact.meta.checksum);
            library.write().await.add(artifact.meta.path, info);
        }

        Ok(assets)
    }

    async fn process_asset(
        &self,
        asset: ImportedAsset,
        config: &AssetConfig,
        library: &SharedLibrary,
    ) -> Result<ImportResult, ImportError> {
        let path = &asset.path;
        let id = asset.id;
        let source = match config.source(path.source()) {
            Some(source) => source,
            None => return Err(ImportError::InvalidSource { path: path.clone() }),
        };

        let meta = match config.registry().get(id.ty()) {
            Some(meta) => meta,
            None => {
                return Err(ImportError::UnRegistered {
                    id,
                    path: path.clone(),
                })
            }
        };

        let artifact = match meta.process(&id, path, source, config.cache()) {
            Some(process) => process
                .await
                .map_err(|e| ImportError::process(path.clone(), e))?,
            None => {
                let temp_path = config.cache().unprocessed_artifact_path(&id);
                config
                    .cache()
                    .load_artifact(&temp_path)
                    .await
                    .map_err(|e| ImportError::process(path.clone(), e))?
            }
        };

        let mut removed_assets = vec![];
        let mut removed_dependencies = vec![];
        if let Ok(prev_meta) = config.cache().load_artifact_meta(&id).await {
            for child in prev_meta.children {
                if !artifact.meta.children.contains(&child) {
                    for path in AssetRefresher.remove_artifact(child, config.cache()).await {
                        library.write().await.remove(&path);
                    }
                    removed_assets.push(child);
                }
            }

            for dep in prev_meta.dependencies {
                if !artifact.meta.dependencies.contains(&dep) {
                    removed_dependencies.push(dep);
                }
            }
        }

        let save_path = config.cache().artifact_path(&id);
        config
            .cache()
            .save_artifact(&save_path, &artifact)
            .await
            .map_err(|e| ImportError::process(path.clone(), e))?;

        let result = ImportResult::new(asset)
            .with_removed_assets(removed_assets)
            .with_removed_dependencies(removed_dependencies);

        Ok(result)
    }

    fn get_process_order(
        &self,
        mut assets: HashMap<AssetId, ImportedAsset>,
    ) -> Vec<Vec<ImportedAsset>> {
        let mut order = vec![];
        while !assets.is_empty() {
            let mut group = vec![];
            for (id, asset) in assets.iter() {
                if asset
                    .dependencies
                    .iter()
                    .all(|dep| !assets.contains_key(dep))
                {
                    group.push(*id);
                }
            }

            if group.is_empty() {
                panic!("Cyclic dependency detected");
            }

            order.push(group.iter().map(|id| assets.remove(id).unwrap()).collect());
        }

        order
    }
}

pub struct ImportResult {
    pub asset: ImportedAsset,
    pub removed_assets: Vec<AssetId>,
    pub removed_dependencies: Vec<AssetId>,
}

impl ImportResult {
    pub fn new(asset: ImportedAsset) -> Self {
        Self {
            asset,
            removed_assets: vec![],
            removed_dependencies: vec![],
        }
    }

    pub fn with_removed_assets(mut self, removed_assets: Vec<AssetId>) -> Self {
        self.removed_assets = removed_assets;
        self
    }

    pub fn with_removed_dependencies(mut self, removed_dependencies: Vec<AssetId>) -> Self {
        self.removed_dependencies = removed_dependencies;
        self
    }
}

pub struct ImportedAsset {
    pub id: AssetId,
    pub path: AssetPath,
    pub dependencies: Vec<AssetId>,
}

impl ImportedAsset {
    pub fn new(id: AssetId, path: AssetPath, dependencies: Vec<AssetId>) -> Self {
        Self {
            id,
            path,
            dependencies,
        }
    }
}

pub struct AssetLoader;

impl AssetLoader {
    pub async fn load(&self, mut paths: HashSet<LoadPath>, database: &AssetDatabase) {
        let config = &database.config;
        let library = &database.library;
        let states = &database.states;
        let actions = &database.actions;

        while !paths.is_empty() {
            let mut dependencies = IndexSet::new();
            let mut deps_loaded = IndexSet::new();
            let mut errors = vec![];

            let loader_paths = std::mem::take(&mut paths);

            for path in loader_paths {
                let result = self.load_asset(&path, config, library, states).await;
                match result {
                    Ok(Some(asset)) => {
                        let id = &asset.meta.id;
                        let deps = asset.meta.dependencies;
                        let parent = asset.meta.parent;

                        if let Some(meta) = config.registry().get(id.ty()) {
                            actions.add(meta.added(*id, asset.asset, None));
                        }

                        for id in states.write().await.loaded(*id, Some(deps), parent) {
                            let meta = match config.registry().get(id.ty()) {
                                Some(meta) => meta,
                                None => continue,
                            };

                            actions.add(meta.loaded(id));
                            deps_loaded.insert(id);
                        }

                        let states = states.read().await;
                        let state = states.get(id).unwrap();
                        for dep in state.dependencies() {
                            if states.get_load_state(*dep).is_unloaded_or_failed() {
                                dependencies.insert(LoadPath::Id(*dep));
                            }
                        }

                        if let Some(parent) = asset
                            .meta
                            .parent
                            .filter(|p| states.get_load_state(*p).is_unloaded_or_failed())
                        {
                            dependencies.insert(LoadPath::Id(parent));
                        }

                        deps_loaded.insert(*id);
                    }
                    Err(error) => {
                        match error {
                            LoadError::Io { id, error, path } => {
                                for id in states.write().await.failed(id) {
                                    let meta = match config.registry().get(id.ty()) {
                                        Some(meta) => meta,
                                        None => continue,
                                    };

                                    actions.add(meta.loaded(id));
                                    deps_loaded.insert(id);
                                }

                                if let Some(meta) = config.registry().get(id.ty()) {
                                    let error = LoadError::Io { id, error, path };
                                    actions.add(meta.failed(id, error));
                                }

                                deps_loaded.insert(id);
                            }
                            LoadError::NotFound { path } => {
                                errors.push(LoadError::NotFound { path })
                            }
                            LoadError::NotRegistered { ty, path } => {
                                errors.push(LoadError::NotRegistered { ty, path })
                            }
                        };
                    }
                    _ => (),
                };

                if errors.len() > 100 {
                    actions.add(BatchEvents::new(errors.drain(..)));
                }
            }

            actions.add(BatchEvents::new(errors.drain(..)));
            paths.extend(dependencies);
        }
    }

    async fn load_asset(
        &self,
        load_path: &LoadPath,
        config: &AssetConfig,
        library: &SharedLibrary,
        states: &SharedStates,
    ) -> Result<Option<ErasedLoadedAsset>, LoadError> {
        let id = match load_path {
            LoadPath::Id(id) => *id,
            LoadPath::Path(path) => match library.read().await.get_id(path) {
                Some(id) => id,
                None => return Err(LoadError::NotFound { path: path.clone() }),
            },
        };

        if states.read().await.get_load_state(id) == LoadState::Loading {
            return Ok(None);
        } else {
            states.write().await.loading(id);
        }

        let meta = match config.registry().get(id.ty()) {
            Some(meta) => meta,
            None => {
                return Err(LoadError::NotRegistered {
                    ty: id.ty(),
                    path: load_path.clone(),
                })
            }
        };

        let path = config.cache().artifact_path(&id);
        let artifact =
            config
                .cache()
                .load_artifact(&path)
                .await
                .map_err(|error| LoadError::Io {
                    id,
                    error,
                    path: load_path.clone(),
                })?;

        let asset = meta.deserialize(artifact).map_err(|error| LoadError::Io {
            id,
            error: error.into(),
            path: load_path.clone(),
        })?;

        Ok(Some(asset))
    }
}

pub struct AssetRefresher;

impl AssetRefresher {
    pub async fn refresh(&self, mode: RefreshMode, database: &AssetDatabase) {
        let config = &database.config;
        let library = &database.library;
        let actions = &database.actions;

        let result = self.refresh_sources(config, library, mode).await;

        let mut unloads = vec![];
        for path in result.removed {
            if let Some(info) = library.write().await.remove(&path) {
                unloads.push(info.id);
            }
        }

        for error in &result.errors {
            if let Some(id) = import_error_id(error, library) {
                let _ = self.remove_artifact(id, config.cache()).await;
                unloads.push(id);
            }
        }

        let _ = config
            .cache()
            .save_library(library.read().await.deref())
            .await;

        if !result.errors.is_empty() {
            actions.add(BatchEvents::new(result.errors));
        }

        if !unloads.is_empty() {
            actions.add(UnloadAssets::new(unloads));
        }

        if !result.imports.is_empty() {
            let mut events = database.events.lock().await;
            events.push_front(DatabaseEvent::Import(result.imports));
        }
    }

    pub async fn refresh_sources(
        &self,
        config: &AssetConfig,
        library: &SharedLibrary,
        mode: RefreshMode,
    ) -> RefreshResult {
        let mut imports = vec![];
        for (name, source) in config.sources().iter() {
            let path = AssetPath::new(name.clone(), PathBuf::new());
            let scans = self
                .scan_folder(path, source, config.cache(), library, mode)
                .await;
            imports.extend(scans);
        }

        let mut result = RefreshResult::default();
        for scan in imports {
            match scan {
                ImportScan::Added(path) | ImportScan::Modified(path) => {
                    result.imports.insert(path);
                }
                ImportScan::Removed(path) => result.removed.push(path),
                ImportScan::Error(error) => result.errors.push(error),
            }
        }

        result
    }

    async fn scan_file(
        &self,
        path: AssetPath,
        source: &AssetSource,
        cache: &AssetCache,
        library: &SharedLibrary,
        mode: RefreshMode,
    ) -> Option<ImportScan> {
        match path.ext() {
            Some("meta") | None => return None,
            _ => {}
        };

        if mode.contains(RefreshMode::FORCE) {
            return Some(ImportScan::Added(path));
        }

        let info = match library.read().await.get(&path) {
            Some(id) => id,
            None => return Some(ImportScan::Added(path)),
        };

        if mode.contains(RefreshMode::SOURCE_MODIFIED) {
            let asset_bytes = match source.read_asset_bytes(path.path()).await {
                Ok(bytes) => bytes,
                Err(error) => return Some(ImportScan::Error(ImportError::import(path, error))),
            };

            let meta_bytes = match source.read_metadata_bytes(path.path()).await {
                Ok(bytes) => bytes,
                Err(_) => return Some(ImportScan::Added(path)),
            };

            let checksum = ArtifactMeta::calculate_checksum(&asset_bytes, &meta_bytes);

            if checksum != info.checksum {
                return Some(ImportScan::Modified(path));
            }
        }

        if mode.contains(RefreshMode::DEPS_MODIFIED) {
            if self
                .did_dependencies_change(info.id, cache, library, true)
                .await
            {
                return Some(ImportScan::Modified(path));
            }
        }

        None
    }

    async fn scan_folder(
        &self,
        asset_path: AssetPath,
        source: &AssetSource,
        cache: &AssetCache,
        library: &SharedLibrary,
        mode: RefreshMode,
    ) -> Vec<ImportScan> {
        let mut paths = match source.read_dir(asset_path.path()).await {
            Ok(paths) => paths,
            Err(error) => return vec![ImportScan::Error(ImportError::import(asset_path, error))],
        };

        let mut scans = vec![];
        let mut children = HashSet::new();
        while let Some(path) = paths.next().await {
            children.insert(path.clone());
            match source.is_dir(&path).await {
                Ok(true) => {
                    let asset_path = AssetPath::new(asset_path.source().clone(), path);
                    scans.extend(
                        Box::pin(self.scan_folder(asset_path, source, cache, library, mode)).await,
                    );
                }
                Ok(false) => {
                    let asset_path = AssetPath::new(asset_path.source().clone(), path);
                    let scan = self
                        .scan_file(asset_path, source, cache, library, mode)
                        .await;
                    scans.extend(scan);
                }
                Err(error) => {
                    let asset_path = AssetPath::new(asset_path.source().clone(), path);
                    scans.push(ImportScan::Error(ImportError::import(asset_path, error)));
                }
            }
        }

        if mode.contains(RefreshMode::SOURCE_MODIFIED) || mode.contains(RefreshMode::FORCE) {
            let mut folder = match source
                .load_metadata::<Folder, Folder>(asset_path.path())
                .await
            {
                Ok(folder) => folder,
                Err(_) => AssetMetadata::default(),
            };

            let removed = folder.set_children(children);
            for path in removed {
                let asset_path = AssetPath::new(asset_path.source().clone(), path);
                scans.push(ImportScan::Removed(asset_path));
            }

            let _ = source.save_metadata(asset_path.path(), &folder).await;
        }

        scans
    }

    async fn did_dependencies_change(
        &self,
        id: AssetId,
        cache: &AssetCache,
        library: &SharedLibrary,
        recursive: bool,
    ) -> bool {
        let meta = match cache.load_artifact_meta(&id).await {
            Ok(meta) => meta,
            Err(_) => return true,
        };

        if let Some(info) = meta.processed {
            let lib = library.read().await;
            for dep in &info.dependencies {
                let checksum = match lib.get_path(&dep.id).and_then(|path| lib.get(path)) {
                    Some(info) => info.checksum,
                    None => return true,
                };

                if checksum != dep.checksum {
                    return true;
                } else if recursive
                    && Box::pin(self.did_dependencies_change(dep.id, cache, library, true)).await
                {
                    return true;
                }
            }
        }

        false
    }

    pub async fn remove_artifact(&self, id: AssetId, cache: &AssetCache) -> Vec<AssetPath> {
        let mut paths = vec![];
        let meta = match cache.load_artifact_meta(&id).await {
            Ok(meta) => meta,
            Err(_) => return paths,
        };

        for child in meta.children {
            paths.extend(Box::pin(self.remove_artifact(child, cache)).await);
        }

        paths.push(meta.path);

        let _ = cache.remove_artifact(&cache.artifact_path(&id)).await;

        paths
    }
}

pub fn import_error_id(error: &ImportError, library: &SharedLibrary) -> Option<AssetId> {
    match error {
        ImportError::InvalidSource { path }
        | ImportError::InvalidExtension { path }
        | ImportError::NoProcessor { path }
        | ImportError::Process { path, .. }
        | ImportError::MissingExtension { path }
        | ImportError::MissingMainAsset { path }
        | ImportError::Import { path, .. } => library.read_blocking().get(path).map(|info| info.id),
        ImportError::MissingPath { id } => Some(*id),
        ImportError::UnRegistered { .. } => None,
    }
}

#[derive(Debug)]
pub enum ImportScan {
    Added(AssetPath),
    Removed(AssetPath),
    Modified(AssetPath),
    Error(ImportError),
}

bitflags::bitflags! {
    #[derive( Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RefreshMode: u32 {
        const SOURCE_MODIFIED = 1 << 0;
        const DEPS_MODIFIED = 1 << 2;
        const FORCE = 1 << 4;
        const FULL = Self::SOURCE_MODIFIED.bits() | Self::DEPS_MODIFIED.bits();
    }
}

impl std::fmt::Debug for RefreshMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_set();
        if self.contains(RefreshMode::SOURCE_MODIFIED) {
            debug.entry(&"SOURCE_MODIFIED");
        }
        if self.contains(RefreshMode::DEPS_MODIFIED) {
            debug.entry(&"DEPS_MODIFIED");
        }
        if self.contains(RefreshMode::FORCE) {
            debug.entry(&"FORCE");
        }
        debug.finish()
    }
}

#[derive(Debug, Default)]
pub struct RefreshResult {
    pub imports: HashSet<AssetPath>,
    pub removed: Vec<AssetPath>,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Folder {
    children: HashSet<PathBuf>,
}

impl Folder {
    pub fn new() -> Self {
        Self {
            children: HashSet::new(),
        }
    }

    pub fn add_child(&mut self, path: PathBuf) {
        self.children.insert(path);
    }

    pub fn remove_child(&mut self, path: &PathBuf) {
        self.children.remove(path);
    }

    pub fn set_children(&mut self, children: HashSet<PathBuf>) -> Vec<PathBuf> {
        let removed = self
            .children
            .difference(&children)
            .cloned()
            .collect::<Vec<_>>();

        self.children = children;

        removed
    }
}

impl Asset for Folder {}
impl Settings for Folder {}

#[cfg(test)]
mod tests {
    use crate::{
        asset::{Asset, AssetMetadata},
        database::{config::AssetConfig, events::AssetEvent, state::LoadState, AssetDatabase},
        importer::{DefaultProcessor, ImportContext, Importer},
        io::{
            cache::AssetCache, source::AssetSourceName, vfs::VirtualFs, AssetIoError, AssetReader,
            FileSystem,
        },
        plugin::{AssetExt, AssetPlugin},
    };
    use ecs::{core::resource::Res, event::Events, world::action::WorldActions};
    use futures_lite::{future::block_on, AsyncReadExt, AsyncWriteExt};
    use game::{ExitGame, Game, GameBuilder, PostInit};
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct PlainText(String);
    impl Asset for PlainText {}

    impl Importer for PlainText {
        type Asset = PlainText;
        type Settings = ();
        type Processor = DefaultProcessor<Self, Self::Settings>;
        type Error = AssetIoError;

        async fn import(
            _ctx: &mut ImportContext<'_, Self::Asset, Self::Settings>,
            reader: &mut dyn AssetReader,
        ) -> Result<Self::Asset, Self::Error> {
            let mut data = String::new();
            reader.read_to_string(&mut data).await?;
            Ok(PlainText(data))
        }

        fn extensions() -> &'static [&'static str] {
            &["txt"]
        }
    }

    trait TestAssetGame {
        fn set_test_cache(&mut self) -> &mut Self;
    }

    impl TestAssetGame for GameBuilder {
        fn set_test_cache(&mut self) -> &mut Self {
            self.resource_mut::<AssetConfig>()
                .set_cache(AssetCache::test());
            self
        }
    }

    fn test_runner(mut game: Game) {
        game.startup();
        let instant = std::time::Instant::now();
        const DURATION: std::time::Duration = std::time::Duration::from_secs(5);
        while instant.elapsed() < DURATION {
            match game.update() {
                Some(ExitGame::Success) => {
                    game.shutdown();
                    return;
                }
                Some(ExitGame::Failure(error)) => panic!("{}", error),
                None => (),
            }
        }

        game.shutdown();
        panic!("Test timed out");
    }

    async fn create_vfs() -> VirtualFs {
        let fs = VirtualFs::new("");
        let mut writer = fs.writer("test.txt".as_ref()).await.unwrap();
        writer.write(b"Hello, World!").await.unwrap();

        let metadata = AssetMetadata::<PlainText, ()>::new(ID, ());
        let metadata = ron::to_string(&metadata).unwrap();
        let mut meta_writer = fs.writer("test.txt.meta".as_ref()).await.unwrap();
        meta_writer.write(metadata.as_bytes()).await.unwrap();

        fs
    }

    const ID: Uuid = Uuid::from_u128(0);

    #[test]
    fn import_asset() {
        let file_system = block_on(create_vfs());
        Game::new()
            .add_plugin(AssetPlugin)
            .register_asset::<PlainText>()
            .add_importer::<PlainText>()
            .add_asset_source(AssetSourceName::Default, file_system)
            .set_runner(test_runner)
            .set_test_cache()
            .observe::<AssetEvent<PlainText>, _>(
                |events: Res<Events<AssetEvent<PlainText>>>,
                 database: Res<AssetDatabase>,
                 actions: &WorldActions| {
                    let library = database.library().read_blocking();
                    for event in events.iter() {
                        match event {
                            AssetEvent::Imported { id } => {
                                let path = library.get_path(id).map(|p| p.path());
                                assert_eq!(path.and_then(|p| p.to_str()), Some("test.txt"));
                                actions.add(ExitGame::Success);
                            }
                            _ => (),
                        }
                    }
                },
            )
            .run();
    }

    #[test]
    fn load_asset() {
        let file_system = block_on(create_vfs());
        Game::new()
            .add_plugin(AssetPlugin)
            .register_asset::<PlainText>()
            .add_importer::<PlainText>()
            .add_asset_source(AssetSourceName::Default, file_system)
            .set_runner(test_runner)
            .set_test_cache()
            .add_systems(PostInit, |db: Res<AssetDatabase>| db.load(["test.txt"]))
            .observe::<AssetEvent<PlainText>, _>(
                |events: Res<Events<AssetEvent<PlainText>>>,
                 database: Res<AssetDatabase>,
                 actions: &WorldActions| {
                    let states = database.states().read_blocking();
                    for event in events.iter() {
                        match event {
                            AssetEvent::Added { id } => {
                                let state = states.get(id).unwrap();
                                assert_eq!(state.state(), LoadState::Loaded);
                                actions.add(ExitGame::Success);
                            }
                            _ => (),
                        }
                    }
                },
            )
            .run();
    }
}
