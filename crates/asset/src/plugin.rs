use crate::{
    asset::{Asset, AssetType, Assets},
    database::{
        config::AssetConfig, events::AssetEvent, update::RefreshMode, AssetDatabase,
        DatabaseInitError,
    },
    importer::{ImportError, Importer, LoadError, Processor},
    io::{local::LocalAssets, source::AssetSourceName, AssetIo},
};
use ecs::{core::resource::ResMut, event::Events};
use futures::executor::block_on;
use game::{GameBuilder, Init, Plugin};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn name(&self) -> &'static str {
        "Asset"
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_resource(AssetConfig::new());
        game.register_event::<ImportError>();
        game.register_event::<LoadError>();
        game.register_event::<DatabaseInitError>();
        game.add_systems(Init, init_asset_database);
    }

    fn finish(&mut self, game: &mut GameBuilder) {
        let mut config = match game.remove_resource::<AssetConfig>() {
            Some(config) => config,
            None => AssetConfig::new(),
        };

        if config.source(&AssetSourceName::Default).is_none() {
            config.add_source::<LocalAssets>(AssetSourceName::Default, LocalAssets::new("assets"));
        }

        let tasks = game.tasks().clone();
        let actions = game.actions().clone();
        game.add_resource(AssetDatabase::new(config, tasks, actions));
    }
}

pub trait AssetExt: 'static {
    fn add_asset_source<I: AssetIo>(
        &mut self,
        name: impl Into<AssetSourceName>,
        io: I,
    ) -> &mut Self;
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn add_importer<I: Importer>(&mut self) -> &mut Self;
    fn set_processor<P: Processor>(&mut self) -> &mut Self;
}

impl AssetExt for GameBuilder {
    fn add_asset_source<I: AssetIo>(
        &mut self,
        name: impl Into<AssetSourceName>,
        io: I,
    ) -> &mut Self {
        let config = self.resource_mut::<AssetConfig>();
        config.add_source::<I>(name, io);
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
    match block_on(database.init()) {
        Ok(_) => {
            database.refresh(RefreshMode::FORCE);
        }
        Err(error) => events.add(error),
    }
}
