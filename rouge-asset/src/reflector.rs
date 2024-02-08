use std::path::{Path, PathBuf};

use rouge_ecs::{bits::AsBytes, observer::Actions, World};

use crate::{
    actions::{LoadAsset, ProcessAsset},
    filesystem::FileSystem,
    metadata::{AssetMetadata, LoadSettings},
    pipeline::{AssetCacher, AssetPipeline},
    storage::{AssetMetadatas, AssetReflectors},
    AssetId, DevMode, Either, HashId, LoadContext,
};

#[derive(Clone)]
pub struct AssetReflector {
    import_asset: fn(&World, &PathBuf) -> Option<AssetId>,
    add_load_asset: fn(&mut Actions, Either<PathBuf, AssetId>),
    add_process_asset: fn(&mut Actions, AssetId),
    extensions: &'static [&'static str],
}

impl AssetReflector {
    pub fn new<A: AssetPipeline>() -> AssetReflector {
        AssetReflector {
            import_asset: move |world, path| {
                let reflectors = world.resource::<AssetReflectors>();
                let metadata_reflector = reflectors.get_metadata::<A::Settings>();
                let filesystem = world.resource::<FileSystem>();
                let id = if let Some(id) = metadata_reflector.load_metadata(world, path) {
                    id
                } else if let Some(id) = metadata_reflector.create_metadata(world, path) {
                    id
                } else {
                    return None;
                };

                let cache_path = Path::new(".cache").join("data").join(&id.0.to_string());
                let cache_metadata = filesystem.metadata(&cache_path);
                let asset_metadata = filesystem.metadata(&path).ok()?;
                let metadata = world.resource::<AssetMetadatas<A::Settings>>().get(id)?;

                match cache_metadata {
                    Ok(cache_metadata) => {
                        if cache_metadata.modified < asset_metadata.modified {
                            AssetReflector::cache_asset::<A>(world, path, &cache_path, metadata)?;
                        }
                    }
                    _ => AssetReflector::cache_asset::<A>(world, path, &cache_path, metadata)?,
                }

                Some(id)
            },
            add_load_asset: |actions, path| match path {
                Either::Left(path) => {
                    actions.add(LoadAsset::<A::Asset>::path(path));
                }
                Either::Right(id) => {
                    actions.add(LoadAsset::<A::Asset>::id(id));
                }
            },
            add_process_asset: |actions, id| {
                actions.add(ProcessAsset::<A::Asset>::new(id));
            },
            extensions: A::extensions(),
        }
    }

    fn cache_asset<A: AssetPipeline>(
        world: &World,
        path: &PathBuf,
        cache_path: &PathBuf,
        metadata: &AssetMetadata<A::Settings>,
    ) -> Option<()> {
        let filesystem = world.resource::<FileSystem>();
        let data = filesystem.read(&path).ok()?;
        let mut ctx = LoadContext::new(path, metadata);
        let asset = A::load(&mut ctx, &data)?;
        let data = if let Some(cacher) = world.try_resource::<AssetCacher<A::Asset>>() {
            cacher.write(&asset)
        } else {
            data
        };

        let depenencies = ctx.dependencies();
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&depenencies.len().to_le_bytes());
        for id in depenencies {
            bytes.extend_from_slice(&id.0.to_le_bytes());
        }

        bytes.extend_from_slice(&data);
        filesystem.write(&cache_path, &bytes).ok()
    }

    pub fn add_load_asset(&self, actions: &mut Actions, path: Either<PathBuf, AssetId>) {
        (self.add_load_asset)(actions, path);
    }

    pub fn add_process_asset(&self, actions: &mut Actions, id: AssetId) {
        (self.add_process_asset)(actions, id);
    }

    pub fn import_asset(&self, world: &World, path: &PathBuf) {
        (self.import_asset)(world, path);
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    pub fn has_extension(&self, extension: &str) -> bool {
        self.extensions.contains(&extension)
    }
}

pub struct AssetMetadataReflector {
    load_metadata: fn(&World, &PathBuf) -> Option<AssetId>,
    create_metadata: fn(&World, &PathBuf) -> Option<AssetId>,
    save_metadata: fn(&FileSystem, &PathBuf) -> Option<AssetId>,
}

impl AssetMetadataReflector {
    pub fn new<S: LoadSettings>(mode: DevMode) -> AssetMetadataReflector {
        AssetMetadataReflector {
            load_metadata: match mode {
                DevMode::Development => |world, path| {
                    let filesystem = world.resource::<FileSystem>();
                    let metapath = path.join(".meta");
                    let string = filesystem.read_str(&metapath).ok()?;
                    let metadata = toml::from_str::<AssetMetadata<S>>(&string).ok()?;
                    let id = metadata.id();
                    world
                        .resource_mut::<AssetMetadatas<S>>()
                        .insert(id, metadata);
                    Some(id)
                },
                DevMode::Release => |world, path| {
                    let filesystem = world.resource::<FileSystem>();
                    let bytes = filesystem.read(&path).ok()?;
                    let metadata = AssetMetadata::<S>::from_bytes(&bytes)?;
                    let id = metadata.id();
                    world
                        .resource_mut::<AssetMetadatas<S>>()
                        .insert(id, metadata);
                    Some(id)
                },
            },
            create_metadata: match mode {
                DevMode::Development => |world, path| {
                    let metadata = AssetMetadata::<S>::new(AssetId::new(), S::default());
                    let id = metadata.id();
                    let string = toml::to_string(&metadata).unwrap();
                    let filesystem = world.resource::<FileSystem>();

                    let metapath = path.join(".meta");
                    filesystem.write_str(&metapath, &string).ok()?;

                    let info_path = Path::new(".cache")
                        .join("lib")
                        .join(HashId::new(&path).to_string());
                    filesystem.write(&info_path, &id.0.to_le_bytes()).ok()?;

                    let release_metapath = Path::new(".cache").join("meta").join(id.0.to_string());
                    let bytes = metadata.to_bytes();
                    filesystem.write(&release_metapath, &bytes).ok()?;

                    let metadatas = world.resource_mut::<AssetMetadatas<S>>();
                    let id = metadata.id();
                    metadatas.insert(id, metadata);

                    Some(id)
                },
                DevMode::Release => |_, _| panic!("Cannot create metadata in release mode"),
            },
            save_metadata: match mode {
                DevMode::Development => |filesystem, path| {
                    let metapath = path.join(".meta");
                    let string = filesystem.read_str(&metapath).ok()?;
                    let metadata = toml::from_str::<AssetMetadata<S>>(&string).ok()?;
                    let id = metadata.id();

                    let bytes = metadata.to_bytes();
                    let release_metapath = Path::new(".cache").join("meta").join(id.0.to_string());
                    filesystem.write(&release_metapath, &bytes).ok()?;

                    let info_path = Path::new(".cache")
                        .join("lib")
                        .join(HashId::new(&path).to_string());
                    filesystem.write(&info_path, &id.0.to_le_bytes()).ok()?;

                    Some(id)
                },
                DevMode::Release => |_, _| panic!("Cannot save metadata in release mode"),
            },
        }
    }

    pub fn load_metadata(&self, world: &World, path: &PathBuf) -> Option<AssetId> {
        (self.load_metadata)(world, path)
    }

    pub fn create_metadata(&self, world: &World, path: &PathBuf) -> Option<AssetId> {
        (self.create_metadata)(world, path)
    }

    pub fn save_metadata(&self, filesystem: &FileSystem, path: &PathBuf) -> Option<AssetId> {
        (self.save_metadata)(filesystem, path)
    }
}
