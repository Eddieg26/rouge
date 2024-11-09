use crate::{
    core::RenderAssets,
    resources::texture::RenderTexture,
    surface::{RenderSurface, RenderSurfaceTexture},
};
use asset::plugin::AssetPlugin;
use ecs::core::resource::{Res, ResMut};
use game::{AppTag, Plugin};
use window::plugin::WindowPlugin;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "Render"
    }

    fn start(&mut self, game: &mut game::GameBuilder) {
        game.add_sub_app::<RenderApp>();
        // game.sub_app_mut::<RenderApp>().add_systems(phase, systems)
    }

    fn run(&mut self, _game: &mut game::GameBuilder) {}

    fn finish(&mut self, _game: &mut game::GameBuilder) {}

    fn dependencies(&self) -> game::Plugins {
        let mut plugins = game::Plugins::default();
        plugins.add(AssetPlugin);
        plugins.add(WindowPlugin);

        plugins
    }
}

pub struct RenderApp;

impl AppTag for RenderApp {
    const NAME: &'static str = "Render";
}

fn set_surface_texture(
    surface: Res<RenderSurface>,
    mut textures: ResMut<RenderAssets<RenderTexture>>,
    mut surface_texture: ResMut<RenderSurfaceTexture>,
) {
    let surface = match surface.texture() {
        Ok(texture) => texture,
        Err(_) => return,
    };

    let texture = RenderTexture::new(None, surface.texture.create_view(&Default::default()));

    textures.add(RenderSurface::ID.to::<RenderTexture>(), texture);

    surface_texture.set(surface);
}
