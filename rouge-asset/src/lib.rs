use metadata::LoadSettings;
use rouge_ecs::{
    macros::Resource,
    system::{ArgItem, SystemArg},
    world::resource::Resource,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

pub mod database;
pub mod filesystem;
pub mod metadata;
pub mod pack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new() -> Self {
        let id = ulid::Ulid::new();
        let mut state = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut state);

        Self(state.finish())
    }

    pub const fn zero() -> Self {
        Self(0)
    }
}

impl From<u64> for AssetId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetType(u64);

impl AssetType {
    pub fn new<A: Asset>() -> Self {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<A>().hash(&mut state);

        Self(state.finish())
    }
}

impl From<u64> for AssetType {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<AssetType> for u64 {
    fn from(value: AssetType) -> Self {
        value.0
    }
}

impl From<std::any::TypeId> for AssetType {
    fn from(value: std::any::TypeId) -> Self {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut state);

        Self(state.finish())
    }
}

pub trait Asset: Send + Sync + 'static {}

impl Asset for () {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevMode {
    Development,
    Release,
}

pub trait AssetPipeline: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: LoadSettings;
    type Arg: SystemArg;
    type Processor: AssetProcessor<Asset = Self::Asset, Arg = Self::Arg, Settings = Self::Settings>;

    fn load(ctx: &mut LoadContext<Self::Settings>, data: &[u8]) -> Option<Self::Asset>;
    fn unload<'a>(
        id: AssetId,
        asset: Self::Asset,
        settings: &'a Self::Settings,
        arg: ArgItem<'a, Self::Arg>,
    );
    fn extensions() -> &'static [&'static str];
}

pub trait AssetProcessor: Send + Sync + 'static {
    type Asset: Asset;
    type Arg: SystemArg;
    type Settings: LoadSettings;

    fn process<'a>(
        id: AssetId,
        asset: &mut Self::Asset,
        settings: &'a Self::Settings,
        arg: ArgItem<'a, Self::Arg>,
    );
}

impl AssetProcessor for () {
    type Asset = ();
    type Arg = ();
    type Settings = ();

    fn process<'a>(
        id: AssetId,
        asset: &mut Self::Asset,
        settings: &'a Self::Settings,
        arg: ArgItem<'a, Self::Arg>,
    ) {
    }
}

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

pub struct LoadContext<'a, S: LoadSettings> {
    dev_mode: DevMode,
    path: PathBuf,
    id: AssetId,
    settings: &'a S,
    dependencies: Vec<AssetId>,
}

impl<'a, S: LoadSettings> LoadContext<'a, S> {
    pub fn new(dev_mode: DevMode, path: PathBuf, id: AssetId, settings: &'a S) -> Self {
        Self {
            dev_mode,
            path,
            id,
            settings,
            dependencies: Vec::new(),
        }
    }

    pub fn dev_mode(&self) -> DevMode {
        self.dev_mode
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        self.settings
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }
}
