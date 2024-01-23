use rouge_game::{game::Game, plugin::Plugin};
use window::Windows;

pub mod actions;
pub mod raw;
pub mod window;

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn start(&mut self, game: &mut Game) {
        game.add_local_resource(Windows::new());
    }
}
