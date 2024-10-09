use crate::{
    asset::Asset,
    import::registry::AssetRegistry,
    io::{AssetSource, AssetSourceConfig, AssetSources, SourceId},
};

pub struct AssetConfig {
    registry: AssetRegistry,
    sources: AssetSources,
}

impl AssetConfig {
    pub fn new() -> Self {
        Self {
            registry: AssetRegistry::new(),
            sources: AssetSources::new(),
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

    pub fn register_asset<A: Asset>(&mut self) {
        self.registry.register::<A>();
    }

    pub fn add_source<C: AssetSourceConfig>(&mut self, id: SourceId, config: C) {
        self.sources.add(id, AssetSource::new(config));
    }
}
