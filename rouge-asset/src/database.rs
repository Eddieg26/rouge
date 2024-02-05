use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::Path,
};

use crate::{
    filesystem::FileSystem,
    metadata::{AssetMetadata, AssetMetadatas},
    Asset, AssetId, AssetPipeline, AssetProcessor, AssetType, Assets, DevMode, LoadContext,
};
use rouge_ecs::{
    macros::Resource,
    system::{IntoSystem, System, SystemArg},
    world::{resource::Resource, World},
};

fn get_metaname(path: &Path) -> String {
    let mut state = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut state);
    let mut name = state.finish().to_string();
    name.push_str(".meta");
    name
}

fn create_load_asset_system<A: AssetPipeline>(dev_mode: DevMode, path: &Path) -> System {
    let path = path.to_path_buf();
    let system = move |assets: &mut Assets<A::Asset>,
                       metadatas: &mut AssetMetadatas<A::Settings>,
                       filesystem: &FileSystem| {
        let metaname = get_metaname(path.as_path());
        let path = Path::new(".meta").join(metaname);

        if filesystem.exists(&path) {
            let metadata = filesystem.read_str(&path).unwrap();
            let metadata = toml::from_str::<AssetMetadata<A::Settings>>(&metadata).ok()?;
            let id = metadata.id();

            let data = filesystem.read(&path).ok()?;
            let mut ctx = LoadContext::new(dev_mode, path.clone(), id, metadata.settings());
            let asset = A::load(&mut ctx, &data)?;
            assets.insert(id, asset);

            metadatas.insert(id, metadata);

            return Some(id);
        }

        None
    };

    let system = System::new(
        move |world| {
            let mut assets = world.resource_mut::<Assets<A::Asset>>();
            let mut metadatas = world.resource_mut::<AssetMetadatas<A::Settings>>();
            let filesystem = world.resource::<FileSystem>();

            system(&mut assets, &mut metadatas, &filesystem);
        },
        vec![],
        vec![],
    );

    system
}

pub struct ErasedAssetPipeline {
    create_metadata: Box<dyn Fn(&Path, &FileSystem) -> Option<AssetId> + Send + Sync>,
    load: Box<dyn Fn(DevMode, &Path, &World) -> Option<AssetId> + Send + Sync>,
    process: Box<dyn Fn(&[AssetId], &World) + Send + Sync>,
    unload: Box<dyn Fn(AssetId, &World) + Send + Sync>,
}

impl ErasedAssetPipeline {
    pub fn new<A: AssetPipeline>() -> Self {
        Self {
            create_metadata: Box::new(|path, filesystem| {
                let metaname = get_metaname(path);
                let path = Path::new(".meta").join(metaname);

                if filesystem.exists(&path) {
                    let metadata = filesystem.read_str(&path).unwrap();
                    if let Some(metadata) =
                        toml::from_str::<AssetMetadata<A::Settings>>(&metadata).ok()
                    {
                        return Some(metadata.id());
                    } else {
                        let id = AssetId::new();
                        let settings = A::Settings::default();
                        let metadata = AssetMetadata::<A::Settings>::new(id, settings);
                        let metadata = toml::to_string(&metadata).ok()?;
                        filesystem.write_str(&path, &metadata).ok()?;
                        return Some(id);
                    }
                } else {
                    let id = AssetId::new();
                    let settings = A::Settings::default();
                    let metadata = AssetMetadata::<A::Settings>::new(id, settings);
                    let metadata = toml::to_string(&metadata).ok()?;
                    filesystem.write_str(&path, &metadata).ok()?;
                    return Some(id);
                }
            }),
            load: Box::new(|dev_mode, path, world| {
                let metaname = get_metaname(path);
                let path = Path::new(".meta").join(metaname);
                let filesystem = world.resource::<FileSystem>();

                if filesystem.exists(&path) {
                    let metadata = filesystem.read_str(&path).unwrap();
                    let metadata = toml::from_str::<AssetMetadata<A::Settings>>(&metadata).ok()?;
                    let id = metadata.id();

                    let data = filesystem.read(&path).ok()?;
                    let mut ctx = LoadContext::new(dev_mode, path.clone(), id, metadata.settings());
                    let asset = A::load(&mut ctx, &data)?;
                    let assets = world.resource_mut::<Assets<A::Asset>>();
                    assets.insert(id, asset);

                    let metadatas = world.resource_mut::<AssetMetadatas<A::Settings>>();
                    metadatas.insert(id, metadata);

                    return Some(id);
                }

                None
            }),
            process: Box::new(|ids, world| {
                let assets = world.resource_mut::<Assets<A::Asset>>();
                let metadatas = world.resource::<AssetMetadatas<A::Settings>>();
                for id in ids {
                    match (assets.get_mut(*id), metadatas.get(*id)) {
                        (Some(asset), Some(metadata)) => {
                            A::Processor::process(
                                *id,
                                asset,
                                metadata.settings(),
                                A::Arg::get(world),
                            );
                        }
                        _ => {}
                    }
                }
            }),
            unload: Box::new(|id, world| {
                let assets = world.resource_mut::<Assets<A::Asset>>();
                let metadata = world.resource_mut::<AssetMetadatas<A::Settings>>();
                match (assets.remove(id), metadata.remove(id)) {
                    (Some(asset), Some(metadata)) => {
                        A::unload(id, asset, &metadata.settings(), A::Arg::get(world));
                    }
                    _ => {}
                }
            }),
        }
    }

    pub fn create_metadata(&self, path: &Path, filesystem: &FileSystem) -> Option<AssetId> {
        (self.create_metadata)(path, filesystem)
    }

    pub fn load(&self, dev_mode: DevMode, path: &Path, world: &World) -> Option<AssetId> {
        (self.load)(dev_mode, path, world)
    }

    pub fn process(&self, ids: &[AssetId], world: &World) {
        (self.process)(ids, world)
    }

    pub fn unload(&self, id: AssetId, world: &World) {
        (self.unload)(id, world)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AssetInfo {
    ty: AssetType,
    path: String,
}

impl AssetInfo {
    pub fn new<A: Asset>(path: impl Into<String>) -> Self {
        Self {
            ty: AssetType::new::<A>(),
            path: path.into(),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct AssetLibrary {
    infos: HashMap<AssetId, AssetInfo>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        Self {
            infos: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, info: AssetInfo) {
        self.infos.insert(id, info);
    }

    pub fn get(&self, id: AssetId) -> Option<&AssetInfo> {
        self.infos.get(&id)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<AssetInfo> {
        self.infos.remove(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.infos.contains_key(&id)
    }
}

impl serde::Serialize for AssetLibrary {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let infos = self
            .infos
            .iter()
            .map(|(id, info)| (id.0.to_string(), info))
            .collect::<HashMap<_, _>>();
        infos.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for AssetLibrary {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let infos = HashMap::<String, AssetInfo>::deserialize(deserializer)?;
        let infos = infos
            .into_iter()
            .map(|(id, info)| (AssetId(id.parse().unwrap()), info))
            .collect();
        Ok(Self { infos })
    }
}

#[derive(Resource)]
pub struct AssetPipelines {
    pipelines: Vec<ErasedAssetPipeline>,
    ext_map: HashMap<String, usize>,
    ty_map: HashMap<AssetType, usize>,
}

impl AssetPipelines {
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
            ext_map: HashMap::new(),
            ty_map: HashMap::new(),
        }
    }

    pub fn register<A: AssetPipeline>(&mut self) {
        let index = self.pipelines.len();
        self.pipelines.push(ErasedAssetPipeline::new::<A>());
        for ext in A::extensions() {
            self.ext_map.insert(ext.to_string(), index);
        }
        self.ty_map.insert(AssetType::new::<A::Asset>(), index);
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&ErasedAssetPipeline> {
        self.ext_map.get(ext).map(|&index| &self.pipelines[index])
    }

    pub fn get_by_ty(&self, ty: AssetType) -> Option<&ErasedAssetPipeline> {
        self.ty_map.get(&ty).map(|&index| &self.pipelines[index])
    }
}
