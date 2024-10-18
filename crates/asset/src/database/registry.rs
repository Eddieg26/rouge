use crate::{
    asset::{AssetId, AssetMetadata},
    importer::{AssetImporter, AssetProcessor, ImportContext, ImportError, ProcessContext, ProcessError},
    io::{
        cache::{Artifact, ArtifactMeta, AssetCache},
        source::{AssetPath, AssetSource},
        AssetFuture,
    },
};
use hashbrown::HashMap;

pub type ImportFn =
    for<'a> fn(&'a AssetPath, &'a AssetSource) -> AssetFuture<'a, Vec<Artifact>, ImportError>;

pub type ProcessFn =
    for<'a> fn(&'a AssetId, &'a AssetSource, &'a AssetCache) -> AssetFuture<'a, Artifact, ProcessError>;

pub struct AssetRegistry {
    importers: HashMap<&'static str, ImportFn>,
    processors: HashMap<&'static str, ProcessFn>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            importers: HashMap::new(),
            processors: HashMap::new(),
        }
    }

    pub fn importer(&self, ext: &str) -> Option<ImportFn> {
        self.importers.get(ext).copied()
    }

    pub fn add_importer<I: AssetImporter>(&mut self) {
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
                            .map_err(|e| ImportError::from(asset_path, e))?;
                        settings
                    }
                };

                let mut ctx = ImportContext::new(asset_path, source, &settings);
                let mut reader = source
                    .reader(path)
                    .await
                    .map_err(|e| ImportError::from(asset_path, e))?;
                let asset = I::import(&mut ctx, reader.as_mut())
                    .await
                    .map_err(|e| ImportError::from(asset_path, e))?;

                let (dependencies, mut artifacts) = ctx.finish();
                let meta = ArtifactMeta::new(*settings.id(), asset_path.clone())
                    .with_dependencies(dependencies)
                    .with_children(artifacts.iter().map(|a| a.meta.id).collect());

                let artifact = Artifact::from_asset(&asset, meta)
                    .map_err(|e| ImportError::from(asset_path, *e))?;

                artifacts.push(artifact);

                Ok(artifacts)
            })
        };

        let processor: ProcessFn = |id, source, cache| {
            Box::pin(async move {
                let mut artifact = match cache.load_temp_artifact(id).await {
                    Ok(artifact) => artifact,
                    Err(error) => return Err(ProcessError::new(*id, error)),
                };

                let metadata = match source
                    .load_metadata::<I::Asset, I::Settings>(artifact.meta.path.path())
                    .await
                {
                    Ok(metadata) => metadata,
                    Err(error) => match artifact.meta.parent.is_some() {
                        true => AssetMetadata::new(*id, I::Settings::default()),
                        false => return Err(ProcessError::new(*id, error)),
                    },
                };

                let mut asset = bincode::deserialize::<I::Asset>(&artifact.data).map_err(|e| {
                    ProcessError::new(*id, std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;

                let ctx = ProcessContext::new(cache, &metadata);
                I::Processor::process(&mut asset, &ctx)
                    .await
                    .map_err(|e| {
                        ProcessError::new(*id, std::io::Error::new(std::io::ErrorKind::Other, e))
                    })?;

                artifact.data = bincode::serialize(&asset).map_err(|e| {
                    ProcessError::new(*id, std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
                Ok(artifact)
            })
        };

        for ext in I::extensions() {
            self.importers.insert(&ext, importer);
            self.processors.insert(&ext, processor);
        }
    }
}
