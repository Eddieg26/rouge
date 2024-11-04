use crate::{
    asset::{Asset, AssetType, Assets},
    database::{
        config::AssetConfig,
        events::{on_asset_event, on_update_assets_modified, AssetEvent, AssetsModified},
        update::RefreshMode,
        AssetDatabase, DatabaseInitError,
    },
    importer::{ImportError, Importer, LoadError, Processor},
    io::{embedded::EmbeddedFs, local::LocalFs, source::AssetSourceName, FileSystem},
};
use ecs::{core::resource::ResMut, event::Events, world::builtin::events::ResourceUpdate};
use futures::executor::block_on;
use game::{GameBuilder, Init, Plugin};
use std::path::PathBuf;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn name(&self) -> &'static str {
        "Asset"
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_resource(AssetConfig::new());
        game.add_resource(AssetsModified::new());
        game.register_event::<ImportError>();
        game.register_event::<LoadError>();
        game.register_event::<DatabaseInitError>();
        game.register_event::<ResourceUpdate<AssetsModified>>();
        game.add_systems(Init, init_asset_database);
        game.observe::<ResourceUpdate<AssetsModified>, _>(on_update_assets_modified);
    }

    fn finish(&mut self, game: &mut GameBuilder) {
        let mut config = match game.remove_resource::<AssetConfig>() {
            Some(config) => config,
            None => AssetConfig::new(),
        };

        if config.source(&AssetSourceName::Default).is_none() {
            config.add_source(AssetSourceName::Default, LocalFs::new("assets"));
        }

        let tasks = game.tasks().clone();
        let actions = game.actions().clone();
        game.add_resource(AssetDatabase::new(config, tasks, actions));
    }
}

pub trait AssetExt: 'static {
    fn add_asset_source<I: FileSystem>(
        &mut self,
        name: impl Into<AssetSourceName>,
        io: I,
    ) -> &mut Self;
    fn embed_assets(&mut self, name: impl Into<AssetSourceName>, assets: EmbeddedFs) -> &mut Self;
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn add_importer<I: Importer>(&mut self) -> &mut Self;
    fn set_processor<P: Processor>(&mut self) -> &mut Self;
}

impl AssetExt for GameBuilder {
    fn add_asset_source<I: FileSystem>(
        &mut self,
        name: impl Into<AssetSourceName>,
        io: I,
    ) -> &mut Self {
        let config = self.resource_mut::<AssetConfig>();
        config.add_source::<I>(name, io);
        self
    }

    fn embed_assets(&mut self, name: impl Into<AssetSourceName>, assets: EmbeddedFs) -> &mut Self {
        let config = self.resource_mut::<AssetConfig>();
        config.embed_assets(name, assets);
        self
    }

    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let registered = {
            let config = self.resource_mut::<AssetConfig>();
            let ty = AssetType::of::<A>();
            if !config.registry().contains(ty) {
                config.registry_mut().register::<A>();
                false
            } else {
                true
            }
        };

        if !registered {
            self.add_resource(Assets::<A>::new());
            self.register_event::<AssetEvent<A>>();
            self.observe::<AssetEvent<A>, _>(on_asset_event::<A>);
        }

        self
    }

    fn add_importer<I: Importer>(&mut self) -> &mut Self {
        self.register_asset::<I::Asset>();

        let config = self.resource_mut::<AssetConfig>();
        config.registry_mut().add_importer::<I>();

        self
    }

    fn set_processor<P: Processor>(&mut self) -> &mut Self {
        self.register_asset::<P::Asset>();

        let config = self.resource_mut::<AssetConfig>();
        config.registry_mut().set_processor::<P>();

        self
    }
}

fn init_asset_database(
    mut database: ResMut<AssetDatabase>,
    mut events: ResMut<Events<DatabaseInitError>>,
) {
    block_on(init(&mut database, &mut events));
}

async fn init(database: &mut AssetDatabase, events: &mut Events<DatabaseInitError>) {
    for (_, source) in database.config().sources().iter() {
        let _ = source.create_dir(&PathBuf::new()).await;
    }

    match database.init().await {
        Ok(_) => database.refresh(RefreshMode::FULL),
        Err(error) => events.add(error),
    }
}
