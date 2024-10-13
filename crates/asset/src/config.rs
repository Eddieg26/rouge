use crate::{
    asset::Asset,
    cache::AssetCache,
    importer::registry::AssetRegistry,
    io::{local::LocalFS, AssetSource, AssetSourceConfig, AssetSources, SourceId},
};
use std::path::{Path, PathBuf};

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
            cache: AssetCache::new(PathBuf::new(), LocalFS),
        }
    }

    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn sources(&self) -> &AssetSources {
        &self.sources
    }

    pub fn source(&self, id: &SourceId) -> Option<&AssetSource> {
        self.sources.get(id)
    }

    pub fn cache(&self) -> &AssetCache {
        &self.cache
    }

    pub fn register_asset<A: Asset>(&mut self) {
        self.registry.register::<A>();
    }

    pub fn add_source<C: AssetSourceConfig>(&mut self, id: impl Into<SourceId>, config: C) {
        self.sources.add(id.into(), config);
    }

    pub fn set_cache<C: AssetSourceConfig>(&mut self, path: impl AsRef<Path>, config: C) {
        self.cache = AssetCache::new(path.as_ref().to_path_buf(), config);
    }
}
