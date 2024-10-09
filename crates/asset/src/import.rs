use crate::{
    asset::{Asset, AssetId, AssetSettings, Settings},
    io::{AssetFuture, AssetReader, AssetSource},
};
use hashbrown::HashSet;
use std::error::Error;

pub struct ImportContext<'a, S: Settings> {
    source: &'a AssetSource,
    settings: &'a AssetSettings<S>,
    dependencies: HashSet<AssetId>,
}

impl<'a, S: Settings> ImportContext<'a, S> {
    pub fn new(source: &'a AssetSource, settings: &'a AssetSettings<S>) -> Self {
        Self {
            source,
            settings,
            dependencies: HashSet::new(),
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

    pub fn finish(self) -> HashSet<AssetId> {
        self.dependencies
    }
}

pub trait AssetImporter {
    type Asset: Asset;
    type Settings: Settings;
    type Error: Error + Send + Sync + 'static;

    fn import<'a>(
        ctx: &'a mut ImportContext<Self::Settings>,
        reader: &'a mut dyn AssetReader,
    ) -> AssetFuture<'a, Self::Asset, Self::Error>;

    fn extensions() -> &'static [&'static str];
}

pub struct ProcessContext<'a, S: Settings> {
    settings: &'a mut AssetSettings<S>,
}

pub trait AssetProcessor {
    type Asset: Asset;
    type Settings: Settings;
    type Importer: AssetImporter<Asset = Self::Asset, Settings = Self::Settings>;
    type Error: Error + Send + Sync + 'static;

    fn process<'a>(
        ctx: &'a mut ProcessContext<'a, Self::Settings>,
        asset: &'a mut Self::Asset,
    ) -> AssetFuture<'a, (), Self::Error>;
}

pub mod registry {
    use crate::{
        asset::{Asset, AssetId, AssetSettings, AssetType, ErasedAsset, ErasedSettings, Settings},
        cache::AssetCache,
        import::ImportContext,
        io::{AssetFuture, AssetIoError, AssetSource, PathExt},
        library::{Artifact, ArtifactMeta},
    };
    use hashbrown::HashMap;
    use std::path::{Path, PathBuf};

    use super::{AssetImporter, AssetProcessor, ProcessContext};

    pub struct ImportedAsset {
        pub asset: ErasedAsset,
        pub settings: ErasedSettings,
        pub meta: ArtifactMeta,
        save_settings: SaveSettings,
    }

    impl ImportedAsset {
        pub fn new<A: Asset, S: Settings>(
            asset: A,
            settings: AssetSettings<S>,
            meta: ArtifactMeta,
        ) -> Self {
            Self {
                asset: ErasedAsset::new(asset),
                settings: ErasedSettings::new(settings),
                meta,
                save_settings: |source, path, settings| {
                    source.save_settings(path, settings.value::<S>())
                },
            }
        }

        pub fn save_settings<'a>(
            &'a self,
            source: &'a AssetSource,
            path: &'a Path,
        ) -> AssetFuture<'a, usize> {
            (self.save_settings)(source, path, &self.settings)
        }
    }

    #[derive(Debug)]
    pub struct ImportError(Box<dyn std::error::Error>);

    impl std::fmt::Display for ImportError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }

    impl<E: std::error::Error + 'static> From<E> for ImportError {
        fn from(e: E) -> Self {
            Self(Box::new(e))
        }
    }

    pub type ImportAsset =
        for<'a> fn(&'a AssetSource, &'a Path) -> AssetFuture<'a, ImportedAsset, ImportError>;

    pub type ProcessAsset = for<'a> fn(&'a mut ImportedAsset) -> AssetFuture<'a, (), ImportError>;

    pub type SaveAsset = for<'a> fn(
        &'a AssetSource,
        &'a AssetCache,
        &'a ImportedAsset,
        ArtifactMeta,
    ) -> AssetFuture<'a, Artifact>;

    pub type LoadAsset =
        for<'a> fn(&'a AssetCache, AssetId) -> AssetFuture<'a, (ErasedAsset, ArtifactMeta)>;

    pub type SaveSettings =
        for<'a> fn(&'a AssetSource, &'a Path, &'a ErasedSettings) -> AssetFuture<'a, usize>;

    pub struct AssetMetadata {
        importers: HashMap<&'static str, ImportAsset>,
        processor: Option<ProcessAsset>,
        save: SaveAsset,
        load: LoadAsset,
    }

    impl AssetMetadata {
        pub fn new<A: Asset>() -> Self {
            Self {
                importers: HashMap::new(),
                processor: None,
                save: |source, cache, asset, metadata| {
                    Box::pin(async move {
                        let id = asset.meta.id;
                        let path = cache.artifact_path(id);
                        let settings_path = source.meta_path(&path);
                        asset.save_settings(source, &settings_path).await?;

                        let asset = bincode::serialize(asset.asset.value::<A>()).unwrap();
                        let artifact = Artifact::new(asset, metadata);
                        cache.save_artifact(id, &artifact).await?;
                        Ok(artifact)
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
            let import: ImportAsset = |source, path| {
                Box::pin(async move {
                    let settings = source
                        .load_settings::<I::Settings>(&source.meta_path(path))
                        .await
                        .map_err(|e| ImportError::from(e))?;

                    let (asset, mut dependencies) = {
                        let mut reader = source.reader(path);
                        let mut ctx = ImportContext::new(source, &settings);
                        let asset = I::import(&mut ctx, reader.as_mut())
                            .await
                            .map_err(|e| ImportError::from(e))?;
                        (asset, ctx.finish())
                    };

                    let ty = AssetType::of::<I::Asset>();
                    let meta =
                        ArtifactMeta::new(*settings.id(), ty, dependencies.drain().collect());

                    Ok(ImportedAsset::new(asset, settings, meta))
                })
            };

            for ext in I::extensions() {
                self.importers.insert(ext, import);
            }
        }

        pub fn set_processor<P: AssetProcessor>(&mut self) {
            self.processor = Some(|asset| {
                Box::pin(async move {
                    let settings = asset.settings.value_mut::<P::Settings>();
                    let asset = asset.asset.value_mut::<P::Asset>();
                    let mut ctx = ProcessContext { settings };
                    P::process(&mut ctx, asset)
                        .await
                        .map_err(|e| ImportError::from(e))
                })
            });
        }

        pub fn import<'a>(
            &self,
            source: &'a AssetSource,
            path: &'a Path,
        ) -> AssetFuture<'a, ImportedAsset, ImportError> {
            let ext = path.extension().and_then(|ext| ext.to_str());
            let import = self
                .importers
                .get(ext.unwrap_or_default())
                .expect("No importer found for extension");

            import(source, path)
        }

        pub fn process<'a>(
            &self,
            asset: &'a mut ImportedAsset,
        ) -> AssetFuture<'a, (), ImportError> {
            if let Some(processor) = self.processor {
                processor(asset)
            } else {
                Box::pin(async { Ok(()) })
            }
        }

        pub fn save<'a>(
            &self,
            source: &'a AssetSource,
            cache: &'a AssetCache,
            asset: &'a ImportedAsset,
            meta: ArtifactMeta,
        ) -> AssetFuture<'a, Artifact> {
            (self.save)(source, cache, asset, meta)
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

        pub fn get(&self, ty: AssetType) -> &AssetMetadata {
            self.metadatas
                .get(&ty)
                .expect(&format!("Asset type not registered: {:?}", ty))
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

        pub fn set_processor<P: AssetProcessor>(&mut self) {
            let ty = self.register::<P::Asset>();
            self.metadatas.get_mut(&ty).unwrap().set_processor::<P>();
        }
    }

    impl AssetSource {
        pub fn meta_path(&self, path: &Path) -> PathBuf {
            path.append_ext("meta")
        }

        pub fn save_settings<'a, S: Settings>(
            &'a self,
            path: &'a Path,
            settings: &'a AssetSettings<S>,
        ) -> AssetFuture<'a, usize> {
            Box::pin(async move {
                let mut writer = self.writer(&self.meta_path(&path));
                let value = toml::to_string(settings).map_err(|e| AssetIoError::from(e))?;
                writer.write(value.as_bytes()).await
            })
        }

        pub fn load_settings<'a, S: Settings>(
            &'a self,
            path: &'a Path,
        ) -> AssetFuture<'a, AssetSettings<S>> {
            Box::pin(async move {
                let mut reader = self.reader(&self.meta_path(&path));
                let mut data = Vec::new();
                reader.read_to_end(&mut data).await?;
                let value = String::from_utf8(data).map_err(|e| AssetIoError::from(e))?;
                toml::from_str(&value).map_err(|e| AssetIoError::from(e))
            })
        }
    }
}
