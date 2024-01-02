use crate::{game::plugin::Plugin, graphics::plugin::GraphicsPlugin};

use self::asset::MaterialObj;

pub mod asset;

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn name(&self) -> &str {
        "material-plugin"
    }

    fn start(&self, game: &mut crate::game::Game) {
        game.add_resource(asset::MaterialRegistry::new())
            .add_asset::<MaterialObj>()
            .add_loader::<MaterialObj>();
    }

    fn dependencies(&self) -> Vec<Box<dyn Plugin>> {
        vec![Box::new(GraphicsPlugin)]
    }
}
