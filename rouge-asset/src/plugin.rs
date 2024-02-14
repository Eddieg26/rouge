use std::path::PathBuf;

use crate::{
    actions::{
        meta::AssetLoaderMetas,
        observers::{on_import_assets, on_load_assets, on_process_assets, on_unload_assets},
        AssetLoaded, ImportAssets, ImportFolder, ImportProcess, LoadAsset, ProcessAsset,
        SettingsLoaded, UnloadAsset,
    },
    config::AssetConfig,
    database::AssetDatabase,
    loader::AssetLoader,
    storage::{AssetSettings, Assets},
    Asset,
};
use rouge_ecs::{observer::Actions, process::StartProcess, IntoSystem};
use rouge_game::{
    game::{Game, GameEnvironment, Init},
    plugin::Plugin,
    Environment,
};

pub struct AssetPlugin {
    asset_path: PathBuf,
    cache_path: PathBuf,
}

impl AssetPlugin {
    pub fn new(asset_path: PathBuf, cache_path: PathBuf) -> Self {
        Self {
            asset_path,
            cache_path,
        }
    }
}

impl Default for AssetPlugin {
    fn default() -> Self {
        Self::new(PathBuf::from("assets"), PathBuf::from("cache"))
    }
}

impl Plugin for AssetPlugin {
    fn start(&mut self, game: &mut rouge_game::game::Game) {
        game.add_resource(AssetConfig::new(
            self.asset_path.clone(),
            self.cache_path.clone(),
        ))
        .add_resource(AssetDatabase::new())
        .add_system(Init, init_assets.before(import_assets))
        .register_action::<ImportFolder>();

        if !game.has_resource::<AssetLoaderMetas>() {
            game.add_resource(AssetLoaderMetas::new());
        }
    }
}

pub fn init_assets(config: &AssetConfig, environment: &GameEnvironment) {
    match **environment {
        Environment::Development => {
            let _ = std::fs::create_dir_all(config.asset_path());
            let _ = std::fs::create_dir_all(PathBuf::from(config.cache_path()).join("lib"));
        }
        _ => (),
    }
}

pub fn import_assets(config: &AssetConfig, environment: &GameEnvironment, mut actions: Actions) {
    match **environment {
        Environment::Development => {
            actions.add(StartProcess::new(ImportProcess::new(vec![config
                .asset_path()
                .clone()])))
        }
        _ => (),
    }
}

pub trait AssetGameExt {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn add_asset_loader<L: AssetLoader>(&mut self) -> &mut Self;
}

impl AssetGameExt for Game {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        self.add_resource(Assets::<A>::new());
        self.register_action::<LoadAsset<A>>();
        self.register_action::<ImportAssets<A>>();
        self.register_action::<UnloadAsset<A>>();
        self.register_action::<ProcessAsset<A>>();
        self.register_action::<AssetLoaded<A>>();

        self
    }

    fn add_asset_loader<L: AssetLoader>(&mut self) -> &mut Self {
        self.register_action::<SettingsLoaded<L::Settings>>();
        self.add_resource(AssetSettings::<L::Settings>::new());
        self.add_observer(on_import_assets::<L>());
        self.add_observer(on_load_assets::<L>());
        self.add_observer(on_process_assets::<L>());
        self.add_observer(on_unload_assets::<L>());

        if let Some(metas) = self.try_resource_mut::<AssetLoaderMetas>() {
            metas.add::<L>();
        } else {
            let mut metas = AssetLoaderMetas::new();
            metas.add::<L>();
            self.add_resource(metas);
        }

        self
    }
}
