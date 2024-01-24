use rouge_game::game::Game;
use rouge_winit::WinitPlugin;

fn main() {
    Game::new().add_plugin(WinitPlugin::new()).run();
}
