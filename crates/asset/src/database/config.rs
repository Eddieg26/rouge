use crate::{
    asset::{Asset, AssetId, AssetMetadata, AssetType},
    importer::{AssetImporter, AssetProcessor, ImportContext, ImportError, ProcessContext},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache},
        source::{AssetPath, AssetSource, AssetSourceName, AssetSources},
        AssetFuture, AssetIo, AssetIoError,
    },
};
use hashbrown::HashMap;
use std::path::Path;

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

    pub fn sources(&self) -> &AssetSources {
        &self.sources
    }

    pub fn source(&self, name: &AssetSourceName) -> Option<&AssetSource> {
        self.sources.get(name)
    }

    pub fn cache(&self) -> &AssetCache {
        &self.cache
    }

    pub fn add_importer<I: AssetImporter>(&mut self) {
        self.registry.add_importer::<I>();
    }

    pub fn add_source<I: AssetIo>(&mut self, name: impl Into<AssetSourceName>, io: I) {
        self.sources.add(name.into(), io);
    }

    pub fn set_cache(&mut self, path: impl AsRef<Path>) {
        self.cache = AssetCache::new(path);
    }
}

pub type ImportFn =
    for<'a> fn(&'a AssetPath, &'a AssetSource) -> AssetFuture<'a, Vec<Artifact>, ImportError>;

pub type ProcessFn =
    for<'a> fn(&'a AssetId, &'a AssetSource, &'a AssetCache) -> AssetFuture<'a, Artifact>;

pub struct AssetMeta {
    importers: Vec<ImportFn>,
    processor: Option<ProcessFn>,
}

impl AssetMeta {
    pub fn new<A: Asset>() -> Self {
        Self {
            importers: vec![],
            processor: None,
        }
    }

    pub fn add_importer<I: AssetImporter>(&mut self) -> usize {
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
                            .map_err(|e| ImportError::import(asset_path, e))?;
                        settings
                    }
                };

                let mut ctx = ImportContext::new(asset_path, source, &settings);
                let mut reader = source
                    .reader(path)
                    .await
                    .map_err(|e| ImportError::import(asset_path, e))?;
                let asset = I::import(&mut ctx, reader.as_mut())
                    .await
                    .map_err(|e| ImportError::import(asset_path, e))?;

                let (dependencies, mut artifacts) = ctx.finish();
                let meta = ArtifactMeta::new(*settings.id(), asset_path.clone())
                    .with_dependencies(dependencies)
                    .with_children(artifacts.iter().map(|a| a.meta.id).collect());

                let artifact = Artifact::from_asset(&asset, meta)
                    .map_err(|e| ImportError::import(asset_path, *e))?;

                artifacts.push(artifact);

                Ok(artifacts)
            })
        };

        let index = self.importers.len();
        self.importers.push(importer);

        index
    }

    pub fn set_processor<P: AssetProcessor>(&mut self) {
        let processor: ProcessFn = |id, source, cache| {
            Box::pin(async move {
                let mut artifact = cache.load_artifact(&cache.temp_artifact_path(id)).await?;

                let metadata = match source
                    .load_metadata::<P::Asset, P::Settings>(artifact.meta.path.path())
                    .await
                {
                    Ok(metadata) => metadata,
                    Err(error) => match artifact.meta.parent.is_some() {
                        true => AssetMetadata::new(*id, P::Settings::default()),
                        false => return Err(error),
                    },
                };

                let mut asset =
                    bincode::deserialize::<P::Asset>(&artifact.data).map_err(AssetIoError::from)?;

                let ctx = ProcessContext::new(cache, &metadata);
                P::process(&mut asset, &ctx).await.map_err(|e| {
                    AssetIoError::from(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;

                artifact.data = bincode::serialize(&asset).map_err(AssetIoError::from)?;
                Ok(artifact)
            })
        };

        self.processor = Some(processor);
    }

    pub fn process<'a>(
        &self,
        id: &'a AssetId,
        source: &'a AssetSource,
        cache: &'a AssetCache,
    ) -> Option<AssetFuture<'a, Artifact>> {
        match &self.processor {
            Some(processor) => Some(processor(id, source, cache)),
            None => None,
        }
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
        source: &'a AssetSource,
        cache: &'a AssetCache,
    ) -> Option<AssetFuture<Artifact>> {
        self.meta.process(id, source, cache)
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

    pub fn get(&self, ty: AssetType) -> Option<&AssetMeta> {
        self.metas.get(&ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<AssetMetaHandle> {
        self.exts
            .get(ext)
            .map(|index| AssetMetaHandle::new(self.metas.get(&index.ty).unwrap(), index.index))
    }

    pub fn has_processor(&self, ty: AssetType) -> bool {
        self.metas
            .get(&ty)
            .and_then(|meta| Some(meta.processor.is_some()))
            .unwrap_or(false)
    }

    pub fn register<A: Asset>(&mut self) -> AssetType {
        let ty = AssetType::of::<A>();
        self.metas.entry(ty).or_insert(AssetMeta::new::<A>());

        ty
    }

    pub fn add_importer<I: AssetImporter>(&mut self) {
        let ty = self.register::<I::Asset>();
        let meta = self.metas.get_mut(&AssetType::of::<I::Asset>()).unwrap();
        let index = meta.add_importer::<I>();

        for ext in I::extensions() {
            self.exts.insert(ext, MetaIndex::new(ty, index));
        }
    }
}
