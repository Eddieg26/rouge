use crate::{
    asset::{Asset, AssetId, AssetSettings, ErasedAsset, Settings},
    io::{AssetFuture, AssetReader, AssetSource},
};
use hashbrown::{HashMap, HashSet};
use std::error::Error;

pub type SubAssets = HashMap<AssetId, ErasedAsset>;

pub struct ImportContext<'a, S: Settings> {
    source: &'a AssetSource,
    settings: &'a AssetSettings<S>,
    dependencies: HashSet<AssetId>,
    sub_assets: SubAssets,
}

impl<'a, S: Settings> ImportContext<'a, S> {
    pub fn new(source: &'a AssetSource, settings: &'a AssetSettings<S>) -> Self {
        Self {
            source,
            settings,
            dependencies: HashSet::new(),
            sub_assets: SubAssets::new(),
        }
    }

    pub fn id(&self) -> &AssetId {
        self.settings.id()
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn source(&self) -> &AssetSource {
        &self.source
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn add_sub_asset<A: Asset>(&mut self, id: AssetId, asset: A) {
        self.sub_assets.insert(id, ErasedAsset::new(asset));
    }

    pub fn finish(self) -> (HashSet<AssetId>, SubAssets) {
        (self.dependencies, self.sub_assets)
    }
}

pub trait AssetImporter {
    type Asset: Asset;
    type Settings: Settings;
    type Processor: AssetProcessor<Asset = Self::Asset, Settings = Self::Settings>;
    type Error: Error + Send + Sync + 'static;

    fn import<'a>(
        ctx: &'a mut ImportContext<Self::Settings>,
        reader: &'a mut dyn AssetReader,
    ) -> AssetFuture<'a, Self::Asset, Self::Error>;

    fn extensions() -> &'static [&'static str];
}

pub struct ProcessContext<'a, S: Settings> {
    settings: &'a mut AssetSettings<S>,
    sub_assets: &'a mut SubAssets,
}

impl<'a, S: Settings> ProcessContext<'a, S> {
    pub fn new(settings: &'a mut AssetSettings<S>, sub_assets: &'a mut SubAssets) -> Self {
        Self {
            settings,
            sub_assets,
        }
    }
}

pub trait AssetProcessor {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn process<'a>(
        ctx: &'a mut ProcessContext<'a, Self::Settings>,
        asset: &'a mut Self::Asset,
    ) -> AssetFuture<'a, (), Self::Error>;
}

pub mod registry {
    use super::{AssetImporter, AssetProcessor, ProcessContext};
    use crate::{
        asset::{Asset, AssetId, AssetSettings, AssetType, ErasedAsset},
        cache::AssetCache,
        importer::ImportContext,
        io::{AssetFuture, AssetSource},
        library::{Artifact, ArtifactMeta},
    };
    use ecs::{
        event::{Event, Events},
        world::action::WorldAction,
    };
    use hashbrown::HashMap;
    use std::path::{Path, PathBuf};

    pub struct ImportedAsset {
        pub asset: ErasedAsset,
        pub meta: ArtifactMeta,
    }

    impl ImportedAsset {
        pub fn new(asset: ErasedAsset, meta: ArtifactMeta) -> Self {
            Self { asset, meta }
        }
    }

    pub struct ImportResult {
        pub main: ImportedAsset,
        pub sub: Vec<ImportedAsset>,
        pub checksum: u32,
    }

    #[derive(Debug)]
    pub struct ImportError {
        path: PathBuf,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    }

    impl ImportError {
        pub fn from<E: std::error::Error + Send + Sync + 'static>(
            path: impl AsRef<Path>,
            error: E,
        ) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                error: Box::new(error),
            }
        }

        pub fn path(&self) -> &Path {
            &self.path
        }

        pub fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            &*self.error
        }

        pub fn missing_extension(path: impl AsRef<Path>) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                error: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Missing extension",
                )),
            }
        }

        pub fn unsupported_extension(path: impl AsRef<Path>) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                error: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unsupported extension",
                )),
            }
        }
    }

    impl std::fmt::Display for ImportError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                f,
                "Import error for {}: {}",
                self.path.display(),
                self.error
            )
        }
    }

    impl<A: AsRef<Path>, E: std::error::Error + Send + Sync + 'static> From<(A, E)> for ImportError {
        fn from((path, error): (A, E)) -> Self {
            Self {
                path: path.as_ref().to_path_buf(),
                error: Box::new(error),
            }
        }
    }

    impl std::error::Error for ImportError {}
    impl Event for ImportError {}

    #[derive(Debug)]
    pub struct SaveError {
        id: AssetId,
        meta: Option<ArtifactMeta>,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    }

    impl SaveError {
        pub fn from<E: std::error::Error + Send + Sync + 'static>(
            id: AssetId,
            error: E,
            meta: Option<ArtifactMeta>,
        ) -> Self {
            Self {
                id,
                meta,
                error: Box::new(error),
            }
        }

        pub fn id(&self) -> &AssetId {
            &self.id
        }

        pub fn meta(&self) -> Option<&ArtifactMeta> {
            self.meta.as_ref()
        }

        pub fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            &*self.error
        }
    }

    impl std::fmt::Display for SaveError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Save error for {:?}: {}", self.id, self.error)
        }
    }

    impl std::error::Error for SaveError {}

    pub type ImportAsset =
        for<'a> fn(&'a Path, &'a AssetSource) -> AssetFuture<'a, ImportResult, ImportError>;

    pub type SaveAsset = for<'a> fn(
        ErasedAsset,
        ArtifactMeta,
        &'a AssetCache,
    )
        -> AssetFuture<'a, (Artifact, Option<ArtifactMeta>), SaveError>;

    pub type LoadAsset =
        for<'a> fn(&'a AssetCache, AssetId) -> AssetFuture<'a, (ErasedAsset, ArtifactMeta)>;

    pub struct AssetMetadata {
        importers: HashMap<&'static str, ImportAsset>,
        save: SaveAsset,
        load: LoadAsset,
    }

    impl AssetMetadata {
        pub fn new<A: Asset>() -> Self {
            Self {
                importers: HashMap::new(),
                save: |asset, meta, cache| {
                    Box::pin(async move {
                        let id = meta.id;
                        let bytes = bincode::serialize(asset.value::<A>()).unwrap();
                        let prev_meta = cache.load_artifact_meta(&id).await.ok();
                        let artifact = Artifact::new(bytes, meta);
                        let result = cache.save_artifact(&artifact).await;
                        if let Err(e) = result {
                            Err(SaveError::from(id, e, prev_meta))
                        } else {
                            Ok((artifact, prev_meta))
                        }
                    })
                },
                load: |cache, id| {
                    Box::pin(async move {
                        let artifact = cache.load_artifact(id).await?;
                        let asset = bincode::deserialize::<A>(artifact.asset()).unwrap();
                        let meta = artifact.meta;
                        Ok((ErasedAsset::new(asset), meta))
                    })
                },
            }
        }

        pub fn add_importer<I: AssetImporter>(&mut self) {
            let import: ImportAsset = |path, source| {
                Box::pin(async move {
                    let mut settings = match source
                        .load_settings::<I::Settings>(&source.settings_path(path))
                        .await
                    {
                        Ok(settings) => settings,
                        Err(_) => AssetSettings::<I::Settings>::new(
                            AssetId::new::<I::Asset>(),
                            I::Settings::default(),
                        ),
                    };

                    let mut reader = source.reader(path);
                    let (mut asset, mut dependencies, mut sub_assets) = {
                        let mut ctx = ImportContext::new(source, &settings);
                        let asset = I::import(&mut ctx, reader.as_mut())
                            .await
                            .map_err(|e| ImportError::from(path, e))?;
                        let (dependencies, sub_assets) = ctx.finish();
                        (asset, dependencies, sub_assets)
                    };

                    let mut ctx = ProcessContext::new(&mut settings, &mut sub_assets);
                    I::Processor::process(&mut ctx, &mut asset)
                        .await
                        .map_err(|e| ImportError::from(path, e))?;

                    source
                        .save_settings(path, &mut settings)
                        .await
                        .map_err(|e| ImportError::from(path, e))?;

                    let id = *settings.id();
                    let mut main_meta = ArtifactMeta::main(id, dependencies.drain().collect());
                    let mut sub = vec![];

                    for (sub_id, sub_asset) in sub_assets {
                        let sub_meta = ArtifactMeta::sub(sub_id, id);
                        main_meta.add_sub_asset(sub_id);
                        sub.push(ImportedAsset::new(sub_asset, sub_meta));
                    }

                    let checksum = {
                        let settings =
                            toml::to_string(&settings).map_err(|e| ImportError::from(path, e))?;
                        let settings = settings.as_bytes();
                        let asset = reader
                            .flush()
                            .await
                            .map_err(|e| ImportError::from(path, e))?;

                        AssetSource::checksum(&asset, settings)
                    };

                    let result = ImportResult {
                        main: ImportedAsset::new(ErasedAsset::new(asset), main_meta),
                        sub,
                        checksum,
                    };

                    Ok(result)
                })
            };

            for ext in I::extensions() {
                self.importers.insert(ext, import);
            }
        }

        pub fn import<'a>(
            &'a self,
            path: &'a Path,
            source: &'a AssetSource,
        ) -> AssetFuture<'a, ImportResult, ImportError> {
            let ext = path.extension().and_then(|ext| ext.to_str());
            match self.importers.get(ext.unwrap_or_default()) {
                Some(import) => import(path, source),
                None => {
                    let error = std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No Importer found for extension",
                    );
                    Box::pin(async move { Err(ImportError::from(path, error)) })
                }
            }
        }

        pub fn save<'a>(
            &self,
            asset: ErasedAsset,
            meta: ArtifactMeta,
            cache: &'a AssetCache,
        ) -> AssetFuture<'a, (Artifact, Option<ArtifactMeta>), SaveError> {
            (self.save)(asset, meta, cache)
        }

        pub fn load<'a>(
            &self,
            cache: &'a AssetCache,
            id: AssetId,
        ) -> AssetFuture<'a, (ErasedAsset, ArtifactMeta)> {
            (self.load)(cache, id)
        }
    }

    pub struct AssetRegistry {
        metadatas: HashMap<AssetType, AssetMetadata>,
        exts: HashMap<&'static str, AssetType>,
    }

    impl AssetRegistry {
        pub fn new() -> Self {
            Self {
                metadatas: HashMap::new(),
                exts: HashMap::new(),
            }
        }

        pub fn get(&self, ty: &AssetType) -> Option<&AssetMetadata> {
            self.metadatas.get(ty)
        }

        pub fn get_by_ext(&self, ext: &str) -> Option<&AssetMetadata> {
            self.exts.get(&ext).and_then(|ty| self.metadatas.get(ty))
        }

        pub fn register<A: Asset>(&mut self) -> AssetType {
            let ty = AssetType::of::<A>();
            if !self.metadatas.contains_key(&ty) {
                self.metadatas.insert(ty, AssetMetadata::new::<A>());
            }

            ty
        }

        pub fn add_importer<I: AssetImporter>(&mut self) {
            let ty = self.register::<I::Asset>();
            self.metadatas.get_mut(&ty).unwrap().add_importer::<I>();

            for ext in I::extensions() {
                self.exts.insert(ext, ty);
            }
        }
    }
}
