use rouge_ecs::macros::Resource;
use std::collections::HashMap;

use crate::{Asset, AssetId, Settings};

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

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.assets.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut A> {
        self.assets.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.assets.remove(id)
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.assets.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.assets.iter_mut()
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }
}

#[derive(Resource)]
pub struct AssetSettings<S: Settings> {
    settings: HashMap<AssetId, S>,
}

impl<S: Settings> AssetSettings<S> {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, settings: S) {
        self.settings.insert(id, settings);
    }

    pub fn get(&self, id: &AssetId) -> Option<&S> {
        self.settings.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetId) -> Option<&mut S> {
        self.settings.get_mut(id)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<S> {
        self.settings.remove(id)
    }

    pub fn clear(&mut self) {
        self.settings.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &S)> {
        self.settings.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut S)> {
        self.settings.iter_mut()
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.settings.contains_key(id)
    }
}
