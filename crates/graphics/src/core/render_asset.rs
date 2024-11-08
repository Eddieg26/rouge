use asset::asset::{Asset, AssetId};
use ecs::{
    core::{resource::Resource, IndexMap},
    system::{ArgItem, SystemArg},
};
use std::{hash::Hash, sync::Arc};

pub trait RenderAsset: 'static {
    type Id: Copy + Eq + Hash + 'static;
}

pub struct RenderAssets<R: RenderAsset> {
    assets: IndexMap<R::Id, R>,
}

impl<R: RenderAsset> RenderAssets<R> {
    pub fn new() -> Self {
        Self {
            assets: IndexMap::new(),
        }
    }

    pub fn add(&mut self, id: R::Id, asset: R) {
        self.assets.insert(id, asset);
    }

    pub fn get(&self, id: &R::Id) -> Option<&R> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &R::Id) -> Option<&mut R> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &R::Id) -> Option<R> {
        self.assets.shift_remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&R::Id, &R)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&R::Id, &mut R)> {
        self.assets.iter_mut()
    }

    pub fn contains(&self, id: R::Id) -> bool {
        self.assets.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn sort_by(&mut self, sorter: impl FnMut(&R::Id, &R, &R::Id, &R) -> std::cmp::Ordering) {
        self.assets.sort_by(sorter);
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

impl<R: RenderAsset<Id: Ord>> RenderAssets<R> {
    pub fn sort_keys(&mut self) {
        self.assets.sort_keys();
    }
}

impl<R: RenderAsset> std::ops::Index<usize> for RenderAssets<R> {
    type Output = R;

    fn index(&self, index: usize) -> &Self::Output {
        &self.assets[index]
    }
}

impl<R: RenderAsset> std::ops::IndexMut<usize> for RenderAssets<R> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.assets[index]
    }
}

impl<R: RenderAsset> Default for RenderAssets<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RenderAsset> Resource for RenderAssets<R> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ReadWrite {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetUsage {
    Keep,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderAssetWorld {
    Main,
    Render,
}

#[derive(Debug, Clone)]
pub enum ExtractError {
    MissingAsset,
    MissingDependency,
    DependencyFailed,
    Error(Arc<dyn std::error::Error + 'static>),
}

impl<E: std::error::Error + 'static> From<E> for ExtractError {
    fn from(error: E) -> Self {
        Self::Error(Arc::new(error))
    }
}

#[allow(unused_variables)]
pub trait RenderAssetExtractor: 'static {
    type Source: Asset;
    type Asset: RenderAsset;
    type Arg: SystemArg;

    fn extract(
        id: &AssetId,
        source: &mut Self::Source,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<Self::Asset, ExtractError>;

    fn update(
        id: &AssetId,
        source: &mut Self::Source,
        asset: &mut Self::Asset,
        arg: &mut ArgItem<Self::Arg>,
    ) -> Result<(), ExtractError> {
        Ok(())
    }

    fn remove(id: &AssetId, assets: &mut RenderAssets<Self::Asset>, arg: &mut ArgItem<Self::Arg>);

    fn usage(id: &AssetId, source: &Self::Source) -> AssetUsage {
        AssetUsage::Keep
    }
}

pub trait RenderResourceExtractor: 'static {
    type Resource: Resource;
    type Arg: SystemArg;

    fn extract(arg: ArgItem<Self::Arg>) -> Result<Self::Resource, ExtractError>;
}
