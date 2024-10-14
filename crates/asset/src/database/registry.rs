use crate::{
    asset::{Asset, AssetSettings, AssetType, ErasedAsset},
    cache::{Artifact, ArtifactMeta, AssetCache},
    importer::{AssetImporter, AssetProcessor, ImportContext, ImportError},
    io::{AssetFuture, AssetSource},
};
use ecs::core::IndexMap;
use hashbrown::HashMap;
use std::path::Path;

pub struct ImportResult {
    pub main: Artifact,
    pub sub: Vec<Artifact>,
}

pub type ImportFn =
    for<'a> fn(&'a Path, &'a AssetSource) -> AssetFuture<'a, ImportResult, ImportError>;

pub type SaveFn = for<'a> fn(&'a Artifact, &'a AssetCache) -> AssetFuture<'a, (), ImportError>;

pub struct AssetMetadata {
    importers: HashMap<&'static str, ImportFn>,
    deserialize: fn(&[u8]) -> Result<ErasedAsset, bincode::Error>,
}

impl AssetMetadata {
    pub fn new<A: Asset>() -> Self {
        Self {
            importers: HashMap::new(),
            deserialize: |bytes| bincode::deserialize::<A>(bytes).map(ErasedAsset::new),
        }
    }

    pub fn add_importer<I: AssetImporter>(&mut self) {
        let importer: ImportFn = |path, source| {
            Box::pin(async move {
                let settings = match source.load_settings::<I::Settings>(path).await {
                    Ok(settings) => settings,
                    Err(_) => AssetSettings::default::<I::Asset>(),
                };

                let mut ctx = ImportContext::new(path, source, &settings);
                let mut reader = source.reader(path);
                let mut asset = I::import(&mut ctx, reader.as_mut())
                    .await
                    .map_err(|e| ImportError::from(path, e))?;
                I::Processor::process(&mut ctx, &mut asset)
                    .await
                    .map_err(|e| ImportError::from(path, e))?;

                source
                    .save_settings(path, &settings)
                    .await
                    .map_err(|e| ImportError::from(path, e))?;

                let (mut dependencies, assets) = ctx.finish();
                let dependencies = dependencies.drain().map(|d| d).collect();
                let sub_ids = assets.iter().map(|s| s.meta.id).collect();
                let meta =
                    ArtifactMeta::main(*settings.id(), path.to_path_buf(), dependencies, sub_ids);

                let bytes = reader
                    .flush()
                    .await
                    .map_err(|e| ImportError::from(path, e))?;
                let settings =
                    toml::to_string(&settings).map_err(|e| ImportError::from(path, e))?;
                let checksum = AssetSource::checksum(&bytes, settings.as_bytes());
                let artifact = Artifact::from_asset(asset, meta, checksum);

                Ok(ImportResult {
                    main: artifact,
                    sub: assets,
                })
            })
        };

        for ext in I::extensions() {
            self.importers.insert(ext, importer);
        }
    }

    pub fn import<'a>(
        &self,
        path: &'a Path,
        source: &'a AssetSource,
    ) -> AssetFuture<'a, ImportResult, ImportError> {
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        if let Some(importer) = self.importers.get(ext) {
            importer(path, source)
        } else {
            Box::pin(async move { Err(ImportError::unsupported_extension(path)) })
        }
    }

    pub fn deserialize(&self, bytes: &[u8]) -> Result<ErasedAsset, bincode::Error> {
        (self.deserialize)(bytes)
    }
}

pub struct AssetRegistry {
    metadatas: IndexMap<AssetType, AssetMetadata>,
    exts: HashMap<&'static str, AssetType>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            metadatas: IndexMap::new(),
            exts: HashMap::new(),
        }
    }

    pub fn get(&self, ty: &AssetType) -> Option<&AssetMetadata> {
        self.metadatas.get(ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&AssetMetadata> {
        self.exts.get(ext).and_then(|ty| self.metadatas.get(ty))
    }

    pub fn register<A: Asset>(&mut self) -> AssetType {
        let ty = AssetType::of::<A>();
        if !self.metadatas.contains_key(&ty) {
            self.metadatas.insert(ty, AssetMetadata::new::<A>());
        }

        ty
    }

    pub fn add_importer<A: AssetImporter>(&mut self) {
        let ty = self.register::<A::Asset>();
        for ext in A::extensions() {
            self.exts.insert(ext, ty);
        }

        let metadata = self.metadatas.get_mut(&ty).unwrap();
        metadata.add_importer::<A>();
    }
}
