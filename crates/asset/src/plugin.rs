use crate::{
    actions::StartAssetAction,
    database::{config::AssetConfig, AssetDatabase},
    importer::ImportError,
};
use ecs::{
    core::resource::{Res, ResMut},
    event::Events,
    task::TaskPool,
    world::action::WorldActions,
};
use game::{GameBuilder, Plugin};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn name(&self) -> &'static str {
        "Asset"
    }

    fn start(&mut self, game: &mut GameBuilder) {
        game.add_resource(AssetConfig::new());
        game.register_event::<StartAssetAction>();
        game.register_event::<ImportError>();
        game.observe::<StartAssetAction, _>(on_start_asset_action);
    }

    fn finish(&mut self, game: &mut GameBuilder) {
        let config = match game.remove_resource::<AssetConfig>() {
            Some(config) => config,
            None => AssetConfig::new(),
        };

        let database = AssetDatabase::new(config);
        game.add_resource(database);
    }
}

fn on_start_asset_action(
    mut events: ResMut<Events<StartAssetAction>>,
    database: Res<AssetDatabase>,
    world_actions: &WorldActions,
    tasks: &TaskPool,
) {
    let mut actions = database.actions_lock();
    actions.extend(events.drain().map(|a| a.take()));
    if !actions.is_running() {
        actions.start();
        let database = database.clone();
        let actions = world_actions.clone();

        tasks.spawn(move || {
            while let Some(mut action) = database.actions_lock().pop() {
                action.execute(&database.config(), &actions);
            }

            database.actions_lock().stop();
        });
    }
}
