use super::events::{
    AssetDepsLoaded, AssetDepsUnloaded, AssetFailed, AssetImported, AssetLoaded, AssetUnloaded,
};
use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetType, ErasedAsset},
    importer::{ImportContext, ImportError, Importer, LoadError, ProcessContext, Processor},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache, ErasedLoadedAsset, ProcessedInfo},
        embedded::EmbeddedFs,
        source::{AssetPath, AssetSource, AssetSourceName, AssetSources},
        AssetFuture, AssetIoError, FileSystem,
    },
};
use ecs::{core::resource::Resource, world::action::WorldActionFn};
use hashbrown::HashMap;

pub struct AssetConfig {
    registry: AssetRegistry,
    sources: AssetSources,
    cache: AssetCache,
}

impl AssetConfig {
    pub fn new() -> Self {
        Self {
            registry: AssetRegistry::new(),
            sources: AssetSources::new(),
            cache: AssetCache::new(".cache"),
        }
    }

    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut AssetRegistry {
        &mut self.registry
    }

    pub fn sources(&self) -> &AssetSources {
        &self.sources
    }

    pub fn source(&self, name: &AssetSourceName) -> Option<&AssetSource> {
        self.sources.get(name)
    }

    pub fn cache(&self) -> &AssetCache {
        &self.cache
    }

    pub fn add_importer<I: Importer>(&mut self) {
        self.registry.add_importer::<I>();
    }

    pub fn add_source<I: FileSystem>(&mut self, name: impl Into<AssetSourceName>, io: I) {
        self.sources.add(name.into(), io);
    }

    pub fn embed_assets(&mut self, name: impl Into<AssetSourceName>, assets: EmbeddedFs) {
        self.sources.add(name.into(), assets);
    }

    #[allow(dead_code)]
    pub(crate) fn set_cache(&mut self, cache: AssetCache) {
        self.cache = cache;
    }
}

impl Resource for AssetConfig {}

pub type ImportFn =
    for<'a> fn(&'a AssetPath, &'a AssetSource) -> AssetFuture<'a, Vec<Artifact>, ImportError>;

pub type ProcessFn = for<'a> fn(
    &'a AssetId,
    &'a AssetPath,
    &'a AssetSource,
    &'a AssetCache,
) -> AssetFuture<'a, Artifact>;

pub struct AssetMeta {
    importers: Vec<ImportFn>,
    processor: Option<ProcessFn>,
    serialize: fn(&ErasedAsset) -> Result<Vec<u8>, bincode::Error>,
    deserialize: fn(Artifact) -> Result<ErasedLoadedAsset, bincode::Error>,
    imported: fn(AssetPath, AssetId) -> WorldActionFn,
    loaded: fn(AssetId, ErasedAsset, Option<Vec<AssetId>>) -> WorldActionFn,
    deps_loaded: fn(AssetId) -> WorldActionFn,
    deps_unloaded: fn(AssetId) -> WorldActionFn,
    unloaded: fn(AssetId) -> WorldActionFn,
    failed: fn(AssetId, LoadError) -> WorldActionFn,
}

impl AssetMeta {
    pub fn new<A: Asset>() -> Self {
        Self {
            importers: vec![],
            processor: None,
            serialize: |asset| match asset.asset::<A>() {
                Some(asset) => bincode::serialize(asset),
                None => Err(bincode::Error::from(bincode::ErrorKind::Custom(format!(
                    "Invalid asset type: Expected {}, Found {:?}",
                    std::any::type_name::<A>(),
                    asset.ty(),
                )))),
            },
            deserialize: |artifact| {
                let asset = artifact.deserialize::<A>()?;
                Ok(ErasedLoadedAsset::new(asset, artifact.meta))
            },
            imported: |path, id| AssetImported::<A>::new(path, id).into(),
            loaded: |id, asset, deps| {
                let loaded = AssetLoaded::<A>::new(id, asset.take());
                if let Some(deps) = deps {
                    loaded.with_dependencies(deps).into()
                } else {
                    loaded.into()
                }
            },
            deps_loaded: |id| AssetDepsLoaded::<A>::new(id).into(),
            deps_unloaded: |id| AssetDepsUnloaded::<A>::new(id).into(),
            unloaded: |id| AssetUnloaded::<A>::new(id).into(),
            failed: |id, error| AssetFailed::<A>::new(id, error).into(),
        }
    }

    pub fn add_importer<I: Importer>(&mut self) -> usize {
        let importer: ImportFn = |asset_path, source| {
            Box::pin(async move {
                let path = asset_path.path();
                let settings = match source.load_metadata::<I::Asset, I::Settings>(path).await {
                    Ok(settings) => settings,
                    Err(_) => {
                        let settings = AssetMetadata::<I::Asset, I::Settings>::default();
                        source
                            .save_metadata(path, &settings)
                            .await
                            .map_err(|e| ImportError::import(asset_path.clone(), e))?;
                        settings
                    }
                };

                let mut ctx = ImportContext::new(asset_path, source, &settings);
                let mut reader = source
                    .reader(path)
                    .await
                    .map_err(|e| ImportError::import(asset_path.clone(), e))?;
                let asset = I::import(&mut ctx, reader.as_mut())
                    .await
                    .map_err(|e| ImportError::import(asset_path.clone(), e))?;

                let checksum = {
                    let asset = source
                        .read_asset_bytes(path)
                        .await
                        .map_err(|e| ImportError::import(asset_path.clone(), e))?;
                    let metadata = source
                        .read_metadata_bytes(path)
                        .await
                        .map_err(|e| ImportError::import(asset_path.clone(), e))?;

                    ArtifactMeta::calculate_checksum(&asset, &metadata)
                };

                let (dependencies, mut artifacts) = ctx.finish();
                let meta = ArtifactMeta::new(*settings.id(), asset_path.clone(), checksum)
                    .with_dependencies(dependencies)
                    .with_children(artifacts.iter().map(|a| a.meta.id).collect());

                let artifact = Artifact::from_asset(&asset, meta)
                    .map_err(|e| ImportError::import(asset_path.clone(), *e))?;

                artifacts.push(artifact);

                Ok(artifacts)
            })
        };

        let index = self.importers.len();
        self.importers.push(importer);

        index
    }

    pub fn set_processor<P: Processor>(&mut self) {
        let processor: ProcessFn = |id, path, source, cache| {
            Box::pin(async move {
                let temp_path = cache.unprocessed_artifact_path(id);
                let mut artifact = cache.load_artifact(&temp_path).await?;
                cache.remove_artifact(&temp_path).await?;

                let metadata = match source
                    .load_metadata::<P::Asset, P::Settings>(path.path())
                    .await
                {
                    Ok(metadata) => metadata,
                    Err(error) => match artifact.meta.parent.is_some() {
                        true => AssetMetadata::new(artifact.id().value(), P::Settings::default()),
                        false => return Err(error),
                    },
                };

                let mut asset =
                    bincode::deserialize::<P::Asset>(&artifact.data).map_err(AssetIoError::from)?;

                let ctx = ProcessContext::new(cache, &metadata);
                P::process(&mut asset, &ctx).await.map_err(|e| {
                    AssetIoError::from(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;

                let info = ProcessedInfo::new().with_dependencies(ctx.finish());
                artifact.meta.processed = Some(info);
                artifact.data = bincode::serialize(&asset).map_err(AssetIoError::from)?;
                Ok(artifact)
            })
        };

        self.processor = Some(processor);
    }

    pub fn process<'a>(
        &self,
        id: &'a AssetId,
        path: &'a AssetPath,
        source: &'a AssetSource,
        cache: &'a AssetCache,
    ) -> Option<AssetFuture<'a, Artifact>> {
        match &self.processor {
            Some(processor) => Some(processor(id, path, source, cache)),
            None => None,
        }
    }

    pub fn serialize(&self, asset: &ErasedAsset) -> Result<Vec<u8>, bincode::Error> {
        (self.serialize)(asset)
    }

    pub fn deserialize(&self, artifact: Artifact) -> Result<ErasedLoadedAsset, bincode::Error> {
        (self.deserialize)(artifact)
    }

    pub fn imported(&self, path: AssetPath, id: AssetId) -> WorldActionFn {
        (self.imported)(path, id)
    }

    pub fn loaded(
        &self,
        id: AssetId,
        asset: ErasedAsset,
        deps: Option<Vec<AssetId>>,
    ) -> WorldActionFn {
        (self.loaded)(id, asset, deps)
    }

    pub fn deps_loaded(&self, id: AssetId) -> WorldActionFn {
        (self.deps_loaded)(id)
    }

    pub fn deps_unloaded(&self, id: AssetId) -> WorldActionFn {
        (self.deps_unloaded)(id)
    }

    pub fn unloaded(&self, id: AssetId) -> WorldActionFn {
        (self.unloaded)(id)
    }

    pub fn failed(&self, id: AssetId, error: LoadError) -> WorldActionFn {
        (self.failed)(id, error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetaIndex {
    ty: AssetType,
    index: usize,
}

impl MetaIndex {
    pub fn new(ty: AssetType, index: usize) -> Self {
        Self { ty, index }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

pub struct AssetMetaHandle<'a> {
    meta: &'a AssetMeta,
    index: usize,
}

impl<'a> AssetMetaHandle<'a> {
    pub fn new(meta: &'a AssetMeta, index: usize) -> Self {
        Self { meta, index }
    }

    pub fn import(
        &'a self,
        path: &'a AssetPath,
        source: &'a AssetSource,
    ) -> AssetFuture<Vec<Artifact>, ImportError> {
        (self.meta.importers[self.index])(path, source)
    }

    pub fn process(
        &'a self,
        id: &'a AssetId,
        path: &'a AssetPath,
        source: &'a AssetSource,
        cache: &'a AssetCache,
    ) -> Option<AssetFuture<Artifact>> {
        self.meta.process(id, path, source, cache)
    }
}

pub struct AssetRegistry {
    metas: HashMap<AssetType, AssetMeta>,
    exts: HashMap<&'static str, MetaIndex>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            metas: HashMap::new(),
            exts: HashMap::new(),
        }
    }

    pub fn contains(&self, ty: AssetType) -> bool {
        self.metas.contains_key(&ty)
    }

    pub fn get(&self, ty: AssetType) -> Option<&AssetMeta> {
        self.metas.get(&ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<AssetMetaHandle> {
        self.exts
            .get(ext)
            .map(|index| AssetMetaHandle::new(self.metas.get(&index.ty).unwrap(), index.index))
    }

    pub fn register<A: Asset>(&mut self) -> AssetType {
        let ty = AssetType::of::<A>();
        self.metas.entry(ty).or_insert(AssetMeta::new::<A>());

        ty
    }

    pub fn add_importer<I: Importer>(&mut self) {
        let ty = self.register::<I::Asset>();
        let meta = self.metas.get_mut(&ty).unwrap();
        let index = meta.add_importer::<I>();

        for ext in I::extensions() {
            self.exts.insert(ext, MetaIndex::new(ty, index));
        }
    }

    pub fn set_processor<P: Processor>(&mut self) {
        let ty = self.register::<P::Asset>();
        let meta = self.metas.get_mut(&ty).unwrap();

        meta.set_processor::<P>();
    }
}
