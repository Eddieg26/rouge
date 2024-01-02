use crate::ecs::{resource::ResourceId, Resource, World};
use std::{
    any::TypeId,
    cell::{Ref, RefMut},
    collections::HashMap,
    path::Path,
    vec,
};

pub type AssetId = ResourceId;
pub type AssetType = TypeId;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SerializedMetadata {
    id: AssetId,
    data: String,
}

impl SerializedMetadata {
    pub fn new(id: AssetId, data: String) -> SerializedMetadata {
        SerializedMetadata { id, data }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn data(&self) -> &str {
        &self.data
    }
}

impl Default for SerializedMetadata {
    fn default() -> Self {
        SerializedMetadata::new(ulid::Ulid::new().into(), String::new())
    }
}

pub trait AssetMetadata:
    'static + serde::Serialize + serde::de::DeserializeOwned + Default
{
}

impl AssetMetadata for () {}

pub trait Asset: 'static {}

pub trait AssetLoader: 'static + IntoErasedAssetLoader {
    type Asset: Asset;
    type Metadata: AssetMetadata;

    fn load(ctx: LoadContext, metadata: Self::Metadata) -> Option<Self::Asset>;
    fn unload(_: LoadContext, _: &Self::Asset) {}
    fn reload(_: LoadContext, _: &Self::Asset) {}
    fn postprocess<'a>(_: LoadContext, _: impl Iterator<Item = (&'a AssetId, &'a Self::Asset)>) {}
    fn extensions() -> &'static [&'static str];
}

pub struct Assets<A: Asset> {
    storage: HashMap<AssetId, A>,
}

impl<A: Asset> Assets<A> {
    pub fn new() -> Assets<A> {
        Assets {
            storage: HashMap::new(),
        }
    }

    pub fn get(&self, id: &AssetId) -> Option<&A> {
        self.storage.get(id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut A> {
        self.storage.get_mut(&id)
    }

    pub fn insert(&mut self, id: AssetId, asset: A) -> Option<A> {
        self.storage.insert(id, asset)
    }

    pub fn remove(&mut self, id: &AssetId) -> Option<A> {
        self.storage.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &A)> {
        self.storage.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&AssetId, &mut A)> {
        self.storage.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    pub fn clear(&mut self) {
        self.storage.clear();
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.storage.contains_key(&id)
    }
}

pub struct LoadContext<'a> {
    id: AssetId,
    path: Option<&'a Path>,
    world: &'a World,
}

impl<'a> LoadContext<'a> {
    pub fn new(id: AssetId, path: Option<&'a Path>, world: &'a World) -> LoadContext<'a> {
        LoadContext { id, path, world }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn path(&self) -> Option<&Path> {
        self.path
    }

    pub fn world(&self) -> &World {
        self.world
    }

    pub fn assets<A: Asset>(&self) -> Ref<Assets<A>> {
        self.world.resource::<Assets<A>>()
    }

    pub fn assets_mut<A: Asset>(&self) -> RefMut<Assets<A>> {
        self.world.resource_mut::<Assets<A>>()
    }

    pub fn resource<T: Resource>(&self) -> Ref<T> {
        self.world.resource::<T>()
    }

    pub fn resource_mut<T: Resource>(&self) -> RefMut<T> {
        self.world.resource_mut::<T>()
    }
}

impl<Loader: AssetLoader> IntoErasedAssetLoader for Loader {
    fn into_erased() -> ErasedAssetLoader {
        ErasedAssetLoader {
            load: |world, path| {
                let mut meta_exits = true;
                let metapath = Path::new(path).with_extension("meta");
                let metadata = std::fs::read_to_string(&metapath).unwrap_or(String::new());
                let serialized_metadata =
                    toml::from_str::<SerializedMetadata>(&metadata).unwrap_or_default();

                let metadata = toml::from_str::<Loader::Metadata>(&serialized_metadata.data)
                    .unwrap_or_else(|_| {
                        meta_exits = false;
                        Loader::Metadata::default()
                    });

                if !meta_exits {
                    let serialized_metadata = SerializedMetadata::new(
                        serialized_metadata.id,
                        toml::to_string(&metadata).unwrap(),
                    );
                    std::fs::write(metapath, toml::to_string(&serialized_metadata).unwrap())
                        .unwrap();
                }

                let ctx = LoadContext::new(serialized_metadata.id, Some(Path::new(path)), world);

                if let Some(asset) = Loader::load(ctx, metadata) {
                    let mut assets = world.resource_mut::<Assets<Loader::Asset>>();
                    let id = path.into();
                    assets.insert(id, asset);
                }
            },
            unload: |world, id| {
                if let Some(asset) = world.resource_mut::<Assets<Loader::Asset>>().remove(id) {
                    let ctx = LoadContext::new(*id, None, world);
                    Self::unload(ctx, &asset);
                }
            },
            reload: |world, id| {
                if let Some(asset) = world.resource::<Assets<Loader::Asset>>().get(id) {
                    let ctx = LoadContext::new(*id, None, world);
                    Self::reload(ctx, asset);
                }
            },
            postprocess: |world| {
                let assets = world.resource::<Assets<Loader::Asset>>();
                let ctx = LoadContext::new(ResourceId::zero(), None, world);
                Self::postprocess(ctx, assets.iter());
            },
        }
    }
}

pub trait IntoErasedAssetLoader {
    fn into_erased() -> ErasedAssetLoader;
}

impl<A: Asset> Resource for Assets<A> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct ErasedAssetLoader {
    load: fn(&World, &str),
    unload: fn(&World, &AssetId),
    reload: fn(&World, &AssetId),
    postprocess: fn(&World),
}

pub struct AssetManager {
    paths: Vec<String>,
    extension_map: HashMap<&'static str, AssetType>,
    loaders: HashMap<AssetType, ErasedAssetLoader>,
}

impl AssetManager {
    pub fn new(path: &str) -> AssetManager {
        AssetManager {
            paths: vec![path.to_owned()],
            extension_map: HashMap::new(),
            loaders: HashMap::new(),
        }
    }

    pub fn add_path(&mut self, path: &str) {
        self.paths.push(path.to_owned());
    }

    pub fn register_loader<L: AssetLoader>(&mut self) {
        let loader = L::into_erased();
        for extension in L::extensions() {
            self.extension_map
                .insert(extension, TypeId::of::<L::Asset>());
        }
        self.loaders.insert(TypeId::of::<L::Asset>(), loader);
    }

    pub fn load(&self, world: &World) {
        for path in &self.paths {
            let _ = self.load_path(world, Path::new(path));
        }
    }

    pub fn load_path(&self, world: &World, path: &Path) -> std::io::Result<()> {
        let read_dir = path.read_dir()?;

        for entry in read_dir {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let _ = self.load_path(world, &path);
            } else if let Some(extension) = path.extension().and_then(|p| p.to_str()) {
                if let Some(asset_type) = self.extension_map.get(extension) {
                    let loader = self
                        .loaders
                        .get(asset_type)
                        .expect("Asset loader not found");
                    (loader.load)(
                        world,
                        path.to_str()
                            .expect(&format!("Failed to convert path to string: {:?}", path)),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn unload<T: Asset>(&self, world: &World, id: &AssetId) {
        let asset_type = TypeId::of::<T>();
        if let Some(loader) = self.loaders.get(&asset_type) {
            (loader.unload)(world, id);
        }
    }

    pub fn reload<T: Asset>(&self, world: &World, id: &AssetId) {
        let asset_type = TypeId::of::<T>();
        if let Some(loader) = self.loaders.get(&asset_type) {
            (loader.reload)(world, id);
        }
    }

    pub fn postprocess(&self, world: &World) {
        for loader in self.loaders.values() {
            (loader.postprocess)(world);
        }
    }
}

pub fn load_assets(world: &World) {
    world.resource::<AssetManager>().load(world);
}

impl Resource for AssetManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
