use crate::{
    database::{AssetDatabase, LoadState},
    filesystem::FileSystem,
    metadata::AssetMetadata,
    pipeline::{AssetCacher, AssetPipeline},
    storage::{AssetMetadatas, AssetReflectors, Assets},
    Asset, AssetId, AssetType, Either, HashId, LoadContext,
};
use rouge_ecs::{
    bits::AsBytes,
    system::{
        observer::{Action, Actions, Observer},
        ArgItem, Cloned, SystemArg,
    },
    world::World,
};
use std::{
    collections::HashSet,
    hash::Hash,
    path::{Path, PathBuf},
};

pub struct LoadAsset<A: Asset> {
    path_or_id: Either<PathBuf, AssetId>,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> LoadAsset<A> {
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self {
            path_or_id: Either::Left(path.into()),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn id(id: AssetId) -> Self {
        Self {
            path_or_id: Either::Right(id),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for LoadAsset<A> {
    type Output = AssetId;

    fn skip(&self, world: &World) -> bool {
        match &self.path_or_id {
            Either::Left(path) => {
                let database = world.resource::<AssetDatabase>();
                if let Some(id) = database.id_from_path(path) {
                    return database.load_state(id) == LoadState::Loaded;
                } else {
                    return false;
                }
            }
            Either::Right(id) => {
                let database = world.resource::<AssetDatabase>();
                return database.load_state(*id) == LoadState::Loaded;
            }
        }
    }

    fn execute(&mut self, world: &mut World) -> Self::Output {
        match &self.path_or_id {
            Either::Left(path) => {
                let filesystem = world.resource::<FileSystem>();
                let info_path = Path::new(".cache")
                    .join("lib")
                    .join(&HashId::new(path).to_string());

                let id = if let Ok(bytes) = filesystem.read(&info_path) {
                    let id = AssetId(u64::from_le_bytes(bytes[0..8].try_into().unwrap()));
                    if world.resource::<AssetDatabase>().load_state(id) == LoadState::Loaded {
                        return id;
                    } else {
                        id
                    }
                } else {
                    panic!("Asset not found in cache");
                };

                id
            }
            Either::Right(id) => id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetLoaded {
    id: AssetId,
    ty: AssetType,
}

impl AssetLoaded {
    pub fn new<A: Asset>(id: AssetId) -> Self {
        Self {
            id,
            ty: AssetType::new::<A>(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> &AssetType {
        &self.ty
    }
}

impl Action for AssetLoaded {
    type Output = Self;

    fn skip(&self, _: &World) -> bool {
        false
    }

    fn execute(&mut self, _: &mut World) -> Self::Output {
        self.clone()
    }
}

pub struct ProcessAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> ProcessAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for ProcessAsset<A> {
    type Output = AssetId;

    fn skip(&self, _: &World) -> bool {
        false
    }

    fn execute(&mut self, _: &mut World) -> Self::Output {
        self.id.clone()
    }
}

pub struct UnloadAsset<A: Asset> {
    id: AssetId,
    _marker: std::marker::PhantomData<A>,
}

impl<A: Asset> UnloadAsset<A> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<A: Asset> Action for UnloadAsset<A> {
    type Output = AssetId;

    fn skip(&self, _: &World) -> bool {
        false
    }

    fn execute(&mut self, _: &mut World) -> Self::Output {
        self.id.clone()
    }
}

pub fn create_load_asset_observer<A: AssetPipeline>() -> Observer<LoadAsset<A::Asset>> {
    let observer = move |ids: &[AssetId],
                         actions: &mut Actions,
                         assets: &mut Assets<A::Asset>,
                         database: &mut AssetDatabase,
                         metadatas: &mut AssetMetadatas<A::Settings>,
                         reflectors: &AssetReflectors,
                         cacher: Option<&AssetCacher<A::Asset>>,
                         filesystem: &FileSystem| {
        for id in ids {
            use std::io::{Error, ErrorKind};
            let cache_path = Path::new(".cache").join("data").join(&id.0.to_string());
            let meta_path = Path::new(".cache").join("meta").join(&id.0.to_string());

            let _ = filesystem
                .read(&meta_path)
                .and_then(|bytes| {
                    AssetMetadata::<A::Settings>::from_bytes(&bytes).ok_or(Error::new(
                        ErrorKind::InvalidData,
                        "Failed to load metadata",
                    ))
                })
                .and_then(|metadata| {
                    let cache = filesystem.read(&cache_path)?;
                    let dep_count = usize::from_le_bytes(cache[0..8].try_into().unwrap());
                    let mut dependencies = HashSet::new();
                    for i in 0..dep_count {
                        let id = u64::from_le_bytes(
                            cache[i * 8 + 8..(i + 1) * 8 + 8].try_into().unwrap(),
                        );
                        dependencies.insert(AssetId(id));
                    }

                    let cache = &cache[8 + dep_count * 8..];

                    database.set_load_state(*id, LoadState::Loading);
                    if let Some(cacher) = cacher {
                        let asset = cacher.read(cache).expect("Failed to read asset from cache");
                        assets.insert(metadata.id(), asset);
                        metadatas.insert(metadata.id(), metadata);
                    } else {
                        let asset =
                            A::load(&mut LoadContext::new(&cache_path, &metadata), cache).ok_or(
                                Error::new(ErrorKind::InvalidData, "Failed to load asset"),
                            )?;
                        assets.insert(metadata.id(), asset);
                        metadatas.insert(metadata.id(), metadata);
                    }

                    for dependency in &dependencies {
                        if database.load_state(*dependency) == LoadState::Loaded
                            || database.load_state(*dependency) == LoadState::Loading
                        {
                            continue;
                        }

                        reflectors
                            .get_asset::<A>()
                            .add_load_asset(actions, Either::Right(*id))
                    }

                    database.set_dependencies(*id, dependencies);
                    database.set_load_state(*id, LoadState::Loaded);
                    actions.add(AssetLoaded::new::<A::Asset>(*id));

                    Ok(())
                });
        }
    };

    Observer::new(
        move |paths, world| {
            let mut actions = world.actions().clone();
            let mut assets = world.resource_mut::<Assets<A::Asset>>();
            let mut metadatas = world.resource_mut::<AssetMetadatas<A::Settings>>();
            let mut database = world.resource_mut::<AssetDatabase>().clone();
            let reflectors = world.resource::<AssetReflectors>();
            let cacher = world.try_resource::<AssetCacher<A::Asset>>();
            let filesystem = world.resource::<FileSystem>();

            observer(
                paths,
                &mut actions,
                &mut assets,
                &mut database,
                &mut metadatas,
                reflectors,
                cacher,
                filesystem,
            );
        },
        vec![],
        vec![],
    )
}

pub fn create_process_observer<A: AssetPipeline>() -> Observer<ProcessAsset<A::Asset>> {
    let observer = move |ids: &[AssetId], args: ArgItem<A::Arg>| {};

    Observer::new(
        move |ids, world| {
            let args = A::Arg::get(world);
            observer(ids, args);
        },
        vec![],
        vec![],
    )
}

pub fn create_unload_asset_observer<A: AssetPipeline>() -> Observer<UnloadAsset<A::Asset>> {
    let observer = move |ids: &[AssetId],
                         assets: &mut Assets<A::Asset>,
                         metadatas: &mut AssetMetadatas<A::Settings>,
                         arg: ArgItem<A::Arg>| {
        for id in ids {
            match (assets.remove(*id), metadatas.remove(*id)) {
                (Some(asset), Some(metadata)) => {
                    A::unload(asset, metadata, &arg);
                }
                _ => {}
            }
        }
    };

    Observer::new(
        move |ids, world| {
            let mut assets = world.resource_mut::<Assets<A::Asset>>();
            let mut metadatas = world.resource_mut::<AssetMetadatas<A::Settings>>();
            let arg = A::Arg::get(world);

            observer(ids, &mut assets, &mut metadatas, arg);
        },
        vec![],
        vec![],
    )
}

pub fn asset_loaded_observer(
    ids: &[AssetLoaded],
    actions: &mut Actions,
    reflectors: &AssetReflectors,
    database: Cloned<AssetDatabase>,
) {
    for loaded in ids {
        if database.is_ready(loaded.id()) {
            reflectors
                .get_asset_type(loaded.ty())
                .add_process_asset(actions, loaded.id())
        }
    }
}
