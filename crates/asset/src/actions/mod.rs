use ecs::world::action::WorldActions;
use crate::database::config::AssetConfig;

pub mod import;

pub trait AssetAction: 'static {
    fn execute(&mut self, config: &AssetConfig, actions: &WorldActions);
}
