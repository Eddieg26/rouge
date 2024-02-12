use super::{
    observers::{on_import_assets, on_load_assets, on_process_assets, on_unload_assets},
    AssetLoader, ProcessAsset,
};
use crate::{
    actions::{ImportAssets, LoadAsset, UnloadAsset},
    loader::AssetCacher,
    Asset, AssetId, AssetPath, AssetType,
};
use rouge_ecs::{
    macros::Resource,
    observer::{Actions, Observers},
    sparse::SparseMap,
    World,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AssetLoaderMeta {
    extensions: &'static [&'static str],
    add_load_action: fn(&mut Actions, AssetPath),
    add_import_action: fn(&mut Actions, Vec<PathBuf>),
    add_unload_action: fn(&mut Actions, AssetPath),
    add_process_asset: fn(&mut Actions, AssetId),
    add_on_import_observer: fn(&mut World),
    add_on_load_observer: fn(&mut World),
    add_on_unload_observer: fn(&mut World),
    add_on_process_observer: fn(&mut World),
    clone_cacher: fn(&World, &mut World),
}

impl AssetLoaderMeta {
    pub fn new<L: AssetLoader>() -> Self {
        Self {
            extensions: L::extensions(),
            add_load_action: |actions, path| {
                actions.add(LoadAsset::<L::Asset>::new(path));
            },
            add_import_action: |actions, paths| {
                actions.add(ImportAssets::<L::Asset>::new(paths));
            },
            add_unload_action: |actions, path| {
                actions.add(UnloadAsset::<L::Asset>::new(path));
            },
            add_process_asset: |actions, id| {
                actions.add(ProcessAsset::<L::Asset>::new(id));
            },
            add_on_import_observer: |world| {
                world.add_observers(
                    Observers::<ImportAssets<L::Asset>>::new().add_system(on_import_assets::<L>()),
                )
            },
            add_on_load_observer: |world| {
                world.add_observers(
                    Observers::<LoadAsset<L::Asset>>::new().add_system(on_load_assets::<L>()),
                )
            },
            add_on_process_observer: |world| {
                world.add_observers(
                    Observers::<ProcessAsset<L::Asset>>::new().add_system(on_process_assets::<L>()),
                )
            },
            add_on_unload_observer: |world| {
                world.add_observers(
                    Observers::<UnloadAsset<L::Asset>>::new().add_system(on_unload_assets::<L>()),
                )
            },

            clone_cacher: |src, dst| {
                if let Some(cacher) = src.try_resource::<AssetCacher<L::Asset>>() {
                    dst.add_resource(cacher.clone());
                }
            },
        }
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    pub fn add_load_action(&self, actions: &mut Actions, path: impl Into<AssetPath>) {
        (self.add_load_action)(actions, path.into())
    }

    pub fn add_import_action(&self, actions: &mut Actions, paths: Vec<PathBuf>) {
        (self.add_import_action)(actions, paths)
    }

    pub fn add_unload_action(&self, actions: &mut Actions, path: impl Into<AssetPath>) {
        (self.add_unload_action)(actions, path.into())
    }

    pub fn add_process_asset(&self, actions: &mut Actions, id: AssetId) {
        (self.add_process_asset)(actions, id)
    }

    pub fn add_on_import_observer(&self, world: &mut World) {
        (self.add_on_import_observer)(world)
    }

    pub fn add_on_load_observer(&self, world: &mut World) {
        (self.add_on_load_observer)(world)
    }

    pub fn add_on_unload_observer(&self, world: &mut World) {
        (self.add_on_unload_observer)(world)
    }

    pub fn add_on_process_observer(&self, world: &mut World) {
        (self.add_on_process_observer)(world)
    }

    pub fn clone_cacher(&self, src: &World, dst: &mut World) {
        (self.clone_cacher)(src, dst)
    }
}

#[derive(Debug, Clone, Resource)]
pub struct AssetLoaderMetas {
    metas: SparseMap<AssetType, AssetLoaderMeta>,
    ext_to_ty: SparseMap<&'static str, AssetType>,
}

impl AssetLoaderMetas {
    pub fn new() -> Self {
        Self {
            metas: SparseMap::new(),
            ext_to_ty: SparseMap::new(),
        }
    }

    pub fn add<L: AssetLoader>(&mut self) {
        let meta = AssetLoaderMeta::new::<L>();
        let ty = AssetType::new::<L::Asset>();

        for &ext in meta.extensions() {
            self.ext_to_ty.insert(ext, ty);
        }

        self.metas.insert(ty, meta);
    }

    pub fn get<A: Asset>(&self) -> Option<&AssetLoaderMeta> {
        let ty = AssetType::new::<A>();
        self.metas.get(&ty)
    }

    pub fn get_by_ty(&self, ty: &AssetType) -> Option<&AssetLoaderMeta> {
        self.metas.get(ty)
    }

    pub fn get_by_ext(&self, ext: &str) -> Option<&AssetLoaderMeta> {
        self.ext_to_ty.get(&ext).and_then(|ty| self.metas.get(ty))
    }

    pub fn get_ty(&self, ext: &str) -> Option<AssetType> {
        self.ext_to_ty.get(&ext).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetType, &AssetLoaderMeta)> {
        self.metas.iter()
    }
}
