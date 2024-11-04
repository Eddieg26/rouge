use crate::transform::{update_transforms, Transform};
use game::{Plugin, PostUpdate};

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn name(&self) -> &'static str {
        "Spatial"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        game.register::<Transform>();
        game.add_systems(PostUpdate, update_transforms);
    }
}
