use crate::{
    config::AssetConfig,
    database::{AssetDatabase, LoadState},
    loader::{AssetCacher, AssetLoader, LoadContext},
    storage::{AssetSettings, Assets},
    AssetId, AssetInfo, AssetMetadata, Either,
};
use rouge_ecs::{
    bits::AsBytes,
    meta::{AccessMeta, AccessType},
    observer::{Actions, Observer},
    ArgItem, SystemArg,
};
use std::{collections::HashSet, path::PathBuf};

use super::{
    meta::AssetLoaderMetas, AssetLoaded, ImportAssets, ImportFolder, LoadAsset, MainWorldActions,
    ProcessAsset, SettingsLoaded, UnloadAsset,
};

pub fn on_import_assets<L: AssetLoader>() -> Observer<ImportAssets<L::Asset>> {
    let callback = |paths: &[Vec<PathBuf>],
                    config: &AssetConfig,
                    cacher: Option<&AssetCacher<L::Asset>>| {
        for row in paths {
            for path in row {
                let meta_path = config.meta_path(path);
                let info_path = config.asset_info_path(path);

                let asset_bytes = match std::fs::read(path) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };

                let (metadata, meta_created) = match config.load_metadata::<L::Settings>(&meta_path)
                {
                    Ok(metadata) => (metadata, false),
                    Err(_) => {
                        let metadata = AssetMetadata::<L::Settings>::new(
                            AssetId::gen(),
                            L::Settings::default(),
                        );
                        match config.save_metadata(&meta_path, &metadata) {
                            Ok(_) => (metadata, true),
                            Err(_) => continue,
                        }
                    }
                };

                let settings_bytes = metadata.settings().to_bytes();
                let checksum =
                    AssetInfo::calculate_checksum::<L::Asset>(&settings_bytes, &asset_bytes);

                let (asset_info, info_created) =
                    match config.load_asset_info::<L::Asset>(&info_path) {
                        Ok(info) => (info, false),
                        Err(_) => {
                            let asset_info =
                                AssetInfo::with_checksum::<L::Asset>(metadata.id(), checksum);
                            match config.save_asset_info::<L::Asset>(&info_path, &asset_info) {
                                Ok(_) => (asset_info, true),
                                Err(_) => {
                                    continue;
                                }
                            }
                        }
                    };

                let cache = meta_created || info_created || asset_info.checksum() != checksum;
                if cache {
                    match cacher {
                        Some(cacher) => {
                            let mut ctx = LoadContext::<L::Settings>::new(&path, &metadata);
                            let asset = L::load(&mut ctx, &asset_bytes).expect(
                                "Failed to load asset. This should not happen if the asset is valid.",
                            );
                            let dependencies = std::mem::take(ctx.dependiencies_mut());
                            let _ = config.cache_asset::<L::Asset>(
                                metadata.id(),
                                Either::Left((&asset, &dependencies, cacher)),
                            );
                        }
                        None => {
                            let _ = config.cache_asset::<L::Asset>(
                                metadata.id(),
                                Either::Right(&asset_bytes),
                            );
                        }
                    }
                }

                if meta_created || asset_info.checksum() != checksum {
                    let _ = config.save_metadata(metadata.id(), &metadata);
                    let _ = config.save_asset_info::<L::Asset>(&info_path, &asset_info);
                }
            }
        }
    };

    let reads = vec![
        AccessType::resource::<AssetConfig>(),
        AccessType::resource::<AssetCacher<L::Asset>>(),
    ];

    let observer = Observer::<ImportAssets<L::Asset>>::new(
        move |paths, world| {
            let config = world.resource::<AssetConfig>();
            let cacher = world.try_resource::<AssetCacher<L::Asset>>();
            callback(paths, config, cacher);
        },
        reads,
        vec![],
    );

    observer
}

pub fn on_load_assets<L: AssetLoader>() -> Observer<LoadAsset<L::Asset>> {
    let callback = |ids: &[AssetId],
                    actions: &mut Actions,
                    mut main_actions: Option<&mut MainWorldActions>,
                    config: &AssetConfig,
                    database: &AssetDatabase,
                    metas: &AssetLoaderMetas,
                    cacher: Option<&AssetCacher<L::Asset>>| {
        for id in ids {
            let state = database.load_state(*id);
            if state == LoadState::Loaded || state == LoadState::Loading {
                continue;
            }

            database.set_load_state(*id, LoadState::Loading);

            let metadata = match config.load_metadata::<L::Settings>(id) {
                Ok(metadata) => metadata,
                Err(_) => {
                    database.set_load_state(*id, LoadState::Failed);
                    continue;
                }
            };

            let asset_path = config.cached_asset_path(id);
            let (asset, dependencies) = match cacher {
                Some(cacher) => match cacher.load(&asset_path) {
                    Ok(asset) => asset,
                    Err(_) => {
                        database.set_load_state(*id, LoadState::Failed);
                        continue;
                    }
                },
                None => {
                    let bytes = match std::fs::read(&asset_path) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            database.set_load_state(*id, LoadState::Failed);
                            continue;
                        }
                    };
                    let mut ctx = LoadContext::<L::Settings>::new(&asset_path, &metadata);
                    match L::load(&mut ctx, &bytes) {
                        Ok(asset) => (asset, std::mem::take(ctx.dependiencies_mut())),
                        Err(_) => {
                            database.set_load_state(*id, LoadState::Failed);
                            continue;
                        }
                    }
                }
            };

            match &mut main_actions {
                Some(main_actions) => {
                    main_actions.add(AssetLoaded::<L::Asset>::new(*id, asset));
                    main_actions.add(SettingsLoaded::<L::Settings>::new(*id, metadata.settings));
                }
                None => {
                    actions.add(AssetLoaded::<L::Asset>::new(*id, asset));
                    actions.add(SettingsLoaded::<L::Settings>::new(*id, metadata.settings));
                }
            }

            database.set_load_state(*id, LoadState::Loaded);
            database.set_dependencies::<L::Asset>(
                *id,
                HashSet::from_iter(dependencies.iter().cloned()),
            );

            for dependency in &dependencies {
                let meta = metas
                    .get_by_ty(dependency.ty())
                    .expect("Missing Asset Loader metadata.");
                meta.add_load_action(actions, dependency.id())
            }

            for dependent in database.dependents(*id) {
                if database.is_ready(*dependent.id()) {
                    let meta = metas
                        .get_by_ty(dependent.ty())
                        .expect("Missing Asset Loader metadata.");
                    match &mut main_actions {
                        Some(main_actions) => meta.add_load_action(main_actions, dependent.id()),
                        None => meta.add_load_action(actions, dependent.id()),
                    }
                }
            }
        }
    };

    let reads = vec![
        AccessType::resource::<AssetConfig>(),
        AccessType::resource::<AssetDatabase>(),
        AccessType::resource::<AssetLoaderMetas>(),
        AccessType::resource::<AssetCacher<L::Asset>>(),
    ];

    let observer = Observer::<LoadAsset<L::Asset>>::new(
        move |ids, world| {
            let mut actions = world.actions().clone();
            let main_actions = world.try_resource_mut::<MainWorldActions>();
            let config = world.resource::<AssetConfig>();
            let database = world.resource::<AssetDatabase>();
            let metas = world.resource::<AssetLoaderMetas>();
            let cacher = world.try_resource::<AssetCacher<L::Asset>>();
            callback(
                ids,
                &mut actions,
                main_actions,
                config,
                database,
                metas,
                cacher,
            );
        },
        reads,
        vec![],
    );

    observer
}

pub fn on_unload_assets<L: AssetLoader>() -> Observer<UnloadAsset<L::Asset>> {
    let callback = |ids: &[AssetId],
                    assets: &mut Assets<L::Asset>,
                    settings: &mut AssetSettings<L::Settings>,
                    database: &AssetDatabase,
                    arg: &ArgItem<L::Arg>| {
        for id in ids {
            database.set_load_state(*id, LoadState::Unloading);

            let asset = assets.remove(id).expect("Missing asset.");
            let settings = settings.remove(id).expect("Missing settings.");
            let metadata = AssetMetadata::new(*id, settings);

            let _ = L::unload(&asset, &metadata, arg);

            database.unload(*id);
        }
    };

    let mut reads = Vec::new();
    let mut writes = Vec::new();

    AccessMeta::pick(&mut reads, &mut writes, &L::Arg::metas());
    writes.extend([
        AccessType::resource::<Assets<L::Asset>>(),
        AccessType::resource::<AssetSettings<L::Settings>>(),
    ]);

    let observer = Observer::<UnloadAsset<L::Asset>>::new(
        move |ids, world| {
            let assets = world.resource_mut::<Assets<L::Asset>>();
            let settings = world.resource_mut::<AssetSettings<L::Settings>>();
            let database = world.resource::<AssetDatabase>();
            let arg = L::Arg::get(world);
            callback(ids, assets, settings, database, &arg);
        },
        reads,
        writes,
    );

    observer
}

pub fn on_process_assets<L: AssetLoader>() -> Observer<ProcessAsset<L::Asset>> {
    let callback = |ids: &[AssetId],
                    assets: &Assets<L::Asset>,
                    settings: &AssetSettings<L::Settings>,
                    arg: &ArgItem<L::Arg>| {
        for id in ids {
            let asset = assets.get(id).expect("Missing asset.");
            let settings = settings.get(id).expect("Missing settings.");

            let _ = L::process(&asset, id, settings, arg);
        }
    };

    let mut reads = Vec::new();
    let mut writes = Vec::new();

    AccessMeta::pick(&mut reads, &mut writes, &L::Arg::metas());
    reads.extend([
        AccessType::resource::<Assets<L::Asset>>(),
        AccessType::resource::<AssetSettings<L::Settings>>(),
    ]);

    let observer = Observer::<ProcessAsset<L::Asset>>::new(
        move |ids, world| {
            let assets = world.resource::<Assets<L::Asset>>();
            let settings = world.resource::<AssetSettings<L::Settings>>();
            let arg = L::Arg::get(world);
            callback(ids, assets, settings, &arg);
        },
        reads,
        writes,
    );

    observer
}

pub fn on_import_folder(paths: &[PathBuf], mut actions: Actions, metas: &AssetLoaderMetas) {
    for path in paths {
        let dir = match std::fs::read_dir(path) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let mut ty_map = std::collections::HashMap::new();
        for entry in dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.is_dir() {
                actions.add(ImportFolder::new(path));
            } else if let Some(extension) = path.extension() {
                let extension = extension.to_str().unwrap();
                if let Some(ty) = metas.get_ty(extension) {
                    ty_map.entry(ty).or_insert_with(Vec::new).push(path);
                }
            }
        }

        for (ty, paths) in ty_map {
            if let Some(meta) = metas.get_by_ty(&ty) {
                meta.add_import_action(&mut actions, paths);
            }
        }
    }
}
