use crate::{asset::{AssetId, AssetKind}, io::SourceId};
use hashbrown::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub sub_assets: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
}

impl ArtifactMeta {
    pub fn new(id: AssetId, dependencies: Vec<AssetId>) -> Self {
        Self {
            id,
            sub_assets: vec![],
            dependencies,
        }
    }

    pub fn add_sub_asset(&mut self, sub_asset: AssetId) {
        self.sub_assets.push(sub_asset);
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
    pub path: PathBuf,
    pub kind: AssetKind,
}

impl SourceAsset {
    pub fn new(checksum: u32, path: PathBuf, kind: AssetKind) -> Self {
        Self {
            checksum,
            path,
            kind,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SourceAssets {
    assets: HashMap<AssetId, SourceAsset>,
    paths: HashMap<PathBuf, AssetId>,
}

impl SourceAssets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: AssetId, asset: SourceAsset) {
        self.paths.insert(asset.path.clone(), id);
        self.assets.insert(id, asset);
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<SourceAsset> {
        if let Some(asset) = self.assets.remove(id) {
            self.paths.remove(&asset.path);
            Some(asset)
        } else {
            None
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&SourceAsset> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut SourceAsset> {
        self.assets.get_mut(id)
    }

    pub fn path_id(&self, path: &Path) -> Option<&AssetId> {
        self.paths.get(path)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn contains_path(&self, path: &Path) -> bool {
        self.paths.contains_key(path)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn ids(&self) -> impl Iterator<Item = &AssetId> + '_ {
        self.assets.keys()
    }

    pub fn paths(&self) -> impl Iterator<Item = (&PathBuf, &AssetId)> + '_ {
        self.paths.iter()
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

    pub fn add_sources(&mut self, id: SourceId) {
        self.sources.insert(id, SourceAssets::new());
    }

    pub fn remove_sources(&mut self, id: &SourceId) {
        self.sources.remove(id);
    }

    pub fn get_sources(&self, id: &SourceId) -> Option<&SourceAssets> {
        self.sources.get(id)
    }

    pub fn get_sources_mut(&mut self, id: &SourceId) -> &mut SourceAssets {
        self.sources.entry(*id).or_insert_with(SourceAssets::new)
    }

    pub fn contains_sources(&self, id: &SourceId) -> bool {
        self.sources.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}
