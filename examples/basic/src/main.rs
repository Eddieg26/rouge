use rouge_ecs::{storage::blob::Blob, system::observer::Actions, world::World};
use rouge_game::game::Game;
use rouge_window::{
    actions::WindowResized,
    window::{WindowId, Windows},
};
use rouge_winit::WinitPlugin;

fn main() {
    // let mut world = World::new();
    // world.add_resource(Actions::new());
    // world.add_local_resource(Windows::new());
    // let mut actions = std::mem::take(world.resource_mut::<Actions>());
    // actions.add(WindowResized::new(WindowId::new(0), 0, 0));

    // actions.execute_actions::<WindowResized>(&mut world);

    // world.resource_mut::<Actions>().append(actions);

    Game::new().add_plugin(WinitPlugin::new()).run();
}
