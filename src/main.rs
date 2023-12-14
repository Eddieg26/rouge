use game::Game;
use graphics::{plugin::GraphicsPlugin, renderer::simple::SimpleRendererPlugin};

pub mod ecs;
pub mod game;
pub mod graphics;
pub mod primitives;
pub mod tree;

fn main() {
    Game::new().add_plugin(SimpleRendererPlugin).run();
}
