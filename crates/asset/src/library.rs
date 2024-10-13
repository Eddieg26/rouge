use crate::{
    asset::{AssetId, AssetKind},
    io::SourceId,
};
use hashbrown::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactMeta {
    pub id: AssetId,
    pub sub_assets: Vec<AssetId>,
    pub dependencies: Vec<AssetId>,
    pub kind: AssetKind,
}

impl ArtifactMeta {
    pub fn main(id: AssetId, dependencies: Vec<AssetId>) -> Self {
        Self {
            id,
            sub_assets: vec![],
            dependencies,
            kind: AssetKind::Main,
        }
    }

    pub fn sub(sub: AssetId, main: AssetId) -> Self {
        Self {
            id: sub,
            sub_assets: vec![],
            dependencies: vec![main],
            kind: AssetKind::Sub,
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
    pub id: AssetId,
}

impl SourceAsset {
    pub fn new(checksum: u32, id: AssetId) -> Self {
        Self { checksum, id }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SourceAssets {
    assets: HashMap<PathBuf, SourceAsset>,
    ids: HashMap<AssetId, PathBuf>,
}

impl SourceAssets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            ids: HashMap::new(),
        }
    }

    pub fn add_main(&mut self, path: PathBuf, asset: SourceAsset) {
        self.ids.insert(asset.id, path.clone());
        self.assets.insert(path, asset);
    }

    pub fn add_sub(&mut self, id: AssetId, path: PathBuf) {
        self.ids.insert(id, path);
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<SourceAsset> {
        let path = self.ids.remove(id)?;
        match self.assets.get(&path).map(|s| &s.id == id) {
            Some(true) => self.assets.remove(&path),
            _ => None,
        }
    }

    pub fn remove_main(&mut self, path: &Path) -> Option<SourceAsset> {
        let source = self.assets.remove(path);
        if let Some(source) = &source {
            self.ids.remove(&source.id);
        }

        source
    }

    pub fn remove_sub(&mut self, id: &AssetId) -> Option<PathBuf> {
        self.ids.remove(id)
    }

    pub fn get(&self, path: &Path) -> Option<&SourceAsset> {
        self.assets.get(path)
    }

    pub fn get_mut(&mut self, path: &Path) -> Option<&mut SourceAsset> {
        self.assets.get_mut(path)
    }

    pub fn id_to_path(&self, id: &AssetId) -> Option<&PathBuf> {
        self.ids.get(id)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.assets.contains_key(path)
    }

    pub fn contains_id(&self, id: &AssetId) -> bool {
        self.ids.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> + '_ {
        self.assets.keys()
    }

    pub fn ids(&self) -> impl Iterator<Item = (&AssetId, &PathBuf)> + '_ {
        self.ids.iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &SourceAsset)> + '_ {
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
