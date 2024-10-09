use crate::{
    asset::{AssetId, AssetType},
    io::SourceId,
};
use hashbrown::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub ty: AssetType,
    pub dependencies: Vec<AssetId>,
}

impl ArtifactMeta {
    pub fn new(id: AssetId, ty: AssetType, dependencies: Vec<AssetId>) -> Self {
        Self {
            id,
            ty,
            dependencies,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactHeader {
    pub meta_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub header: ArtifactHeader,
    pub meta: ArtifactMeta,
    asset: Vec<u8>,
}

impl Artifact {
    pub fn new(asset: Vec<u8>, meta: ArtifactMeta) -> Self {
        let meta_size = bincode::serialized_size(&meta).unwrap() as u32;
        Self {
            header: ArtifactHeader { meta_size },
            meta,
            asset,
        }
    }

    pub fn header(&self) -> &ArtifactHeader {
        &self.header
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn asset(&self) -> &[u8] {
        &self.asset
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceAsset {
    pub checksum: u32,
    pub ty: AssetType,
    pub path: PathBuf,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SourceAssets {
    assets: HashMap<AssetId, SourceAsset>,
}

impl SourceAssets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: AssetId, asset: SourceAsset) {
        self.assets.insert(id, asset);
    }

    pub fn remove(&mut self, id: &AssetId) {
        self.assets.remove(id);
    }

    pub fn get(&self, id: &AssetId) -> Option<&SourceAsset> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut SourceAsset> {
        self.assets.get_mut(id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn ids(&self) -> hashbrown::hash_map::Keys<'_, AssetId, SourceAsset> {
        self.assets.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &SourceAsset)> + '_ {
        self.assets.iter()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AssetLibrary {
    sources: HashMap<SourceId, SourceAssets>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        let mut sources = HashMap::new();
        sources.insert(SourceId::Default, SourceAssets::new());
        Self { sources }
    }

    pub fn add_source(&mut self, id: SourceId) {
        self.sources.insert(id, SourceAssets::new());
    }

    pub fn remove_source(&mut self, id: &SourceId) {
        self.sources.remove(id);
    }

    pub fn get_source(&self, id: &SourceId) -> Option<&SourceAssets> {
        self.sources.get(id)
    }

    pub fn get_source_mut(&mut self, id: &SourceId) -> Option<&mut SourceAssets> {
        self.sources.get_mut(id)
    }

    pub fn contains_source(&self, id: &SourceId) -> bool {
        self.sources.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}
