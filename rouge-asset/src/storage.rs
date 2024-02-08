use rouge_ecs::{macros::Resource, world::resource::Resource};
use std::collections::HashMap;

use crate::{
    metadata::{AssetMetadata, LoadSettings},
    pipeline::AssetPipeline,
    reflector::{AssetMetadataReflector, AssetReflector},
    Asset, AssetId, AssetType,
};

#[derive(Resource)]
pub struct Assets<A: Asset> {
    assets: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, asset: A) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: AssetId) -> Option<&A> {
        self.assets.get(&id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut A> {
        self.assets.get_mut(&id)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<A> {
        self.assets.remove(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.assets.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }
}

#[derive(Resource)]
pub struct AssetMetadatas<S: LoadSettings> {
    metadata: HashMap<AssetId, AssetMetadata<S>>,
}

impl<S: LoadSettings> AssetMetadatas<S> {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, metadata: AssetMetadata<S>) {
        self.metadata.insert(id, metadata);
    }

    pub fn get(&self, id: AssetId) -> Option<&AssetMetadata<S>> {
        self.metadata.get(&id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut AssetMetadata<S>> {
        self.metadata.get_mut(&id)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<AssetMetadata<S>> {
        self.metadata.remove(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.metadata.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetMetadata<S>)> {
        self.metadata.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut AssetMetadata<S>)> {
        self.metadata.iter_mut()
    }
}

#[derive(Resource)]
pub struct AssetReflectors {
    asset_reflectors: Vec<AssetReflector>,
    asset_reflectors_map: HashMap<AssetType, usize>,
    extension_reflectors: HashMap<String, usize>,
    metadata_reflectors: HashMap<AssetType, AssetMetadataReflector>,
}

impl AssetReflectors {
    pub fn new() -> Self {
        Self {
            asset_reflectors: Vec::new(),
            asset_reflectors_map: HashMap::new(),
            extension_reflectors: HashMap::new(),
            metadata_reflectors: HashMap::new(),
        }
    }

    pub fn register_asset<A: AssetPipeline>(&mut self, reflector: AssetReflector) {
        let asset_type = AssetType::new::<A::Asset>();
        let index = self.asset_reflectors.len();
        self.asset_reflectors.push(reflector);
        self.asset_reflectors_map.insert(asset_type, index);
        for ext in A::extensions() {
            self.extension_reflectors.insert(ext.to_string(), index);
        }
    }

    pub fn get_asset<A: AssetPipeline>(&self) -> &AssetReflector {
        let asset_type = AssetType::new::<A::Asset>();
        self.get_asset_type(&asset_type)
    }

    pub fn get_asset_type(&self, asset_type: &AssetType) -> &AssetReflector {
        self.asset_reflectors_map
            .get(asset_type)
            .and_then(|index| self.asset_reflectors.get(*index))
            .expect(&format!("No asset reflector found for asset type: {:?}", asset_type)[..])
    }

    pub fn get_asset_extension(&self, ext: &str) -> &AssetReflector {
        self.extension_reflectors
            .get(ext)
            .and_then(|index| self.asset_reflectors.get(*index))
            .expect(&format!("No asset reflector found for extension: {}", ext)[..])
    }

    pub fn register_metadata<S: LoadSettings>(&mut self, reflector: AssetMetadataReflector) {
        self.metadata_reflectors
            .insert(AssetType::new_settings::<S>(), reflector);
    }

    pub fn get_metadata<S: LoadSettings>(&self) -> &AssetMetadataReflector {
        self.metadata_reflectors
            .get(&AssetType::new_settings::<S>())
            .expect(
                &format!(
                    "No metadata reflector found for asset type: {:?}",
                    AssetType::new_settings::<S>()
                )[..],
            )
    }
}
