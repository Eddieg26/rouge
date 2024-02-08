use filesystem::FileSystem;
use metadata::{AssetMetadata, LoadSettings};
use rouge_ecs::{
    macros::Resource,
    world::{resource::Resource, World},
};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use storage::AssetReflectors;

pub mod actions;
pub mod database;
pub mod filesystem;
pub mod metadata;
pub mod pipeline;
pub mod reflector;
pub mod storage;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashId(u64);

impl HashId {
    pub fn new<H: Hash>(hashable: &H) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hashable.hash(&mut hasher);

        Self(hasher.finish())
    }
}

impl ToString for HashId {
    fn to_string(&self) -> String {
        self.0.to_string()
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

    pub fn new_settings<S: LoadSettings>() -> Self {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<S>().hash(&mut state);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum DevMode {
    Development,
    Release,
}

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

pub struct LoadContext<'a, S: LoadSettings> {
    path: &'a Path,
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: LoadSettings> LoadContext<'a, S> {
    pub fn new(path: &'a PathBuf, metadata: &'a AssetMetadata<S>) -> Self {
        Self {
            path,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }
}

pub fn import_assets(world: &World) {
    import_assets_inner(world, Path::new(""));
}

fn import_assets_inner(world: &World, path: &Path) {
    let filesystem = world.resource::<FileSystem>();
    let reflectors = world.resource::<AssetReflectors>();

    let items = filesystem.list(&path).unwrap();
    for item in &items {
        let meta = filesystem.metadata(&path).unwrap();
        if meta.is_dir {
            import_assets_inner(world, &path);
        } else {
            let ext = item.extension().unwrap().to_str().unwrap();
            let reflector = reflectors.get_asset_extension(ext);
            reflector.import_asset(world, item);
        }
    }
}
