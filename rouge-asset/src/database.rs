use crate::{
    filesystem::FileSystem,
    pack::{AsBytes, AssetPack},
    Asset, AssetId, AssetLoader, AssetMetadata, AssetMetadatas, AssetProcessor, AssetSerializer,
    AssetType, Assets, DevMode, LoadContext,
};
use rouge_ecs::{
    macros::Resource,
    system::SystemArg,
    world::{resource::Resource, World},
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

pub struct ErasedAssetLoader {
    load: Box<dyn Fn(DevMode, Box<dyn AssetReader>, PathBuf, &World) -> AssetId + Send + Sync>,
    unload: Box<dyn Fn(AssetId, &World) + Send + Sync>,
    process: Box<dyn Fn(&World) + Send + Sync>,
    extensions: Vec<String>,
    ty: AssetType,
}

impl ErasedAssetLoader {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            load: Box::new(|dev_mode, reader, path, world| match dev_mode {
                DevMode::Development => {
                    let metadata = if let Some(data) = reader.read_metadata() {
                        let string = std::str::from_utf8(data).unwrap();
                        toml::from_str::<AssetMetadata<L::Settings>>(string).unwrap()
                    } else {
                        let id = AssetId::new();
                        let settings = L::Settings::default();
                        let metadata = AssetMetadata::new(id, settings);

                        reader.write_metadata(toml::to_string(&metadata).unwrap().as_bytes());

                        metadata
                    };

                    let id = metadata.id();
                    let settings = metadata.settings();

                    let ctx = LoadContext::new(dev_mode, path.clone(), id, settings);
                    let data = reader.read().unwrap();
                    let asset = L::load(ctx, data);

                    if let Some(serializer) = world.try_resource_mut::<AssetSerializer<L::Asset>>()
                    {
                        let data = (serializer.serialize)(&asset);
                        // TODO Write Pack to data path
                    }

                    world.resource_mut::<Assets<L::Asset>>().insert(id, asset);
                    world
                        .resource_mut::<AssetMetadatas<L::Settings>>()
                        .insert(id, metadata);

                    world.resource_mut::<AssetLibrary>().insert(
                        id,
                        path,
                        AssetType::new::<L::Asset>(),
                    );

                    id
                }
                DevMode::Release => {
                    let metadata = reader.read_metadata().unwrap();
                    let metadata = toml::from_str::<AssetMetadata<L::Settings>>(
                        std::str::from_utf8(metadata).unwrap(),
                    )
                    .unwrap();

                    let id = metadata.id();
                    let data = reader.read().unwrap();

                    if let Some(serializer) = world.try_resource_mut::<AssetSerializer<L::Asset>>()
                    {
                        let asset = (serializer.deserialize)(data);
                        world.resource_mut::<Assets<L::Asset>>().insert(id, asset);
                        world
                            .resource_mut::<AssetMetadatas<L::Settings>>()
                            .insert(id, metadata);
                    } else {
                        let ctx = LoadContext::new(dev_mode, path.clone(), id, metadata.settings());
                        let asset = L::load(ctx, data);
                        world.resource_mut::<Assets<L::Asset>>().insert(id, asset);
                        world
                            .resource_mut::<AssetMetadatas<L::Settings>>()
                            .insert(id, metadata);
                    }

                    id
                }
            }),
            unload: Box::new(|id, world| {
                let assets = world.resource_mut::<Assets<L::Asset>>();
                let metadatas = world.resource_mut::<AssetMetadatas<L::Settings>>();
                match (assets.remove(id), metadatas.remove(id)) {
                    (Some(asset), Some(metadata)) => {
                        L::unload(id, asset, metadata.settings(), L::Arg::get(world))
                    }
                    _ => {}
                }
            }),
            process: Box::new(|world| {
                let assets = world.resource_mut::<Assets<L::Asset>>();
                for (id, asset) in assets.iter_mut() {
                    L::Processor::process(*id, asset, L::Arg::get(world));
                }
            }),
            extensions: L::extensions().iter().map(|s| s.to_string()).collect(),
            ty: AssetType::new::<L::Asset>(),
        }
    }

    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }
}

pub struct AssetLoaders {
    loaders: Vec<ErasedAssetLoader>,
    ext_to_loader: HashMap<String, usize>,
    ty_to_loader: HashMap<AssetType, usize>,
}

impl AssetLoaders {
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
            ext_to_loader: HashMap::new(),
            ty_to_loader: HashMap::new(),
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        let loader = ErasedAssetLoader::new::<L>();
        let index = self.loaders.len();
        for ext in loader.extensions() {
            self.ext_to_loader.insert(ext.clone(), index);
        }

        self.ty_to_loader.insert(loader.ty(), index);

        self.loaders.push(loader);
    }

    pub fn get<A: Asset>(&self) -> Option<&ErasedAssetLoader> {
        self.get_by_type(AssetType::new::<A>())
    }

    pub fn get_by_type(&self, ty: AssetType) -> Option<&ErasedAssetLoader> {
        self.ty_to_loader
            .get(&ty)
            .map(|index| &self.loaders[*index])
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&ErasedAssetLoader> {
        self.ext_to_loader
            .get(ext)
            .map(|index| &self.loaders[*index])
    }
}

#[derive(Resource, serde::Serialize, serde::Deserialize)]
pub struct AssetLibrary {
    path_to_id: HashMap<u64, AssetId>,
    id_to_type: HashMap<AssetId, AssetType>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        Self {
            path_to_id: HashMap::new(),
            id_to_type: HashMap::new(),
        }
    }

    pub fn load(&mut self, fs: Box<dyn FileSystem>, path: impl AsRef<Path>) {
        let contents = fs.read(path.as_ref()).unwrap();
        let loaded = AssetLibrary::from_bytes(&contents);
        let _ = std::mem::replace(self, loaded);
    }

    pub fn insert(&mut self, id: AssetId, path: impl AsRef<Path>, ty: AssetType) {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        path.as_ref().hash(&mut state);
        let path_hash = state.finish();
        self.path_to_id.insert(path_hash, id);
        self.id_to_type.insert(id, ty);
    }

    pub fn get(&self, path: impl AsRef<Path>) -> Option<AssetId> {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        path.as_ref().hash(&mut state);
        let path_hash = state.finish();

        self.path_to_id.get(&path_hash).copied()
    }

    pub fn get_type(&self, id: AssetId) -> Option<AssetType> {
        self.id_to_type.get(&id).copied()
    }

    pub fn remove(&mut self, id: AssetId) -> Option<u64> {
        let path =
            self.path_to_id.iter().find_map(
                |(path, id_)| {
                    if *id_ == id {
                        Some(path.clone())
                    } else {
                        None
                    }
                },
            )?;

        self.path_to_id.remove(&path);
        self.id_to_type.remove(&id);

        Some(path)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.id_to_type.contains_key(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetType, &u64)> {
        self.path_to_id
            .iter()
            .map(|(path, id)| (id, self.id_to_type.get(id).unwrap(), path))
    }
}

#[derive(Resource)]
pub struct AssetDatabase {
    loaders: AssetLoaders,
    root: PathBuf,
    mode: DevMode,
}

impl AssetDatabase {
    pub fn new(root: PathBuf, mode: DevMode) -> Self {
        Self {
            loaders: AssetLoaders::new(),
            root,
            mode,
        }
    }

    pub fn register<L: AssetLoader>(&mut self) {
        self.loaders.register::<L>();
    }

    pub fn load(&self, path: impl AsRef<Path>, world: &World) -> Option<AssetId> {
        todo!()
    }
}

pub trait AssetReader {
    fn load(&mut self, path: &Path);
    fn read(&self) -> Option<&[u8]>;
    fn read_metadata(&self) -> Option<&[u8]>;
    fn write(&self, data: &[u8]);
    fn write_metadata(&self, data: &[u8]);
}

pub struct PackAssetReader {
    fs: Box<dyn FileSystem>,
    path: PathBuf,
    pack: Option<AssetPack>,
}

impl PackAssetReader {
    pub fn new(fs: Box<dyn FileSystem>, path: PathBuf) -> Self {
        Self {
            fs,
            path,
            pack: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<'a> AssetReader for PackAssetReader {
    fn load(&mut self, path: &Path) {
        let pack = self.fs.read(path).unwrap();
        self.pack = Some(AssetPack::from_bytes(&pack));
    }

    fn read(&self) -> Option<&[u8]> {
        match self.pack.as_ref() {
            Some(pack) => Some(pack.data()),
            None => None,
        }
    }

    fn read_metadata(&self) -> Option<&[u8]> {
        match self.pack.as_ref() {
            Some(pack) => Some(pack.metadata()),
            None => None,
        }
    }

    fn write(&self, _: &[u8]) {}

    fn write_metadata(&self, _: &[u8]) {}
}

pub struct RawAssetReader {
    fs: Box<dyn FileSystem>,
    path: PathBuf,
    data: Option<Vec<u8>>,
    metadata: Option<Vec<u8>>,
}

impl RawAssetReader {
    pub fn new(fs: Box<dyn FileSystem>, path: PathBuf) -> Self {
        Self {
            fs,
            path,
            data: None,
            metadata: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AssetReader for RawAssetReader {
    fn load(&mut self, path: &Path) {
        self.data = self.fs.read(path).ok();

        let metapath = self.path.as_os_str().to_str().unwrap().to_owned() + ".meta";
        self.metadata = self.fs.read(Path::new(&metapath)).ok()
    }

    fn read(&self) -> Option<&[u8]> {
        match self.data.as_ref() {
            Some(data) => Some(data),
            None => None,
        }
    }

    fn read_metadata(&self) -> Option<&[u8]> {
        match self.metadata.as_ref() {
            Some(metadata) => Some(metadata),
            None => None,
        }
    }

    fn write(&self, data: &[u8]) {
        let _ = self.fs.write(&self.path, data);
    }

    fn write_metadata(&self, data: &[u8]) {
        let metapath = self.path.as_os_str().to_str().unwrap().to_owned() + ".meta";
        let _ = self.fs.write(Path::new(&metapath), data);
    }
}

// Asset Plugin Features
// Store Assets
// Create Asset Metadata
// Load Asset Metadata
// Load Assets from raw files
// Create Asset Packs
// Load Assets from packs
// Load Assets from network
// Load Assets from memory
// Load Assets Asynchronously
// Load Asset dependencies
// Process Assets
// Serialize/Deserialize Assets
// Unload Assets
// Unload Asset dependencies
// Unload Asset Metadata
// Reload Assets
// Reload Asset dependencies
