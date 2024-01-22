use game::Game;

pub mod engine;
pub mod game;
pub mod plugin;
pub mod time;

pub struct TestPluginA;
pub struct TestPluginB;

impl plugin::Plugin for TestPluginA {
    fn start(&mut self, plugins: &mut plugin::Plugins) {
        println!("TestPluginA::start");
        plugins.register(TestPluginB);
    }

    fn run(&mut self, game: &mut game::Game) {
        println!("TestPluginA::run");
    }

    fn finish(&mut self, game: &mut game::Game) {
        println!("TestPluginA::finish");
    }

    fn dependencies(&self) -> Vec<plugin::PluginId> {
        vec![plugin::PluginId::new::<TestPluginB>()]
    }
}

impl plugin::Plugin for TestPluginB {
    fn start(&mut self, plugins: &mut plugin::Plugins) {
        println!("TestPluginB::start");
    }

    fn run(&mut self, game: &mut game::Game) {
        println!("TestPluginB::run");
    }

    fn finish(&mut self, game: &mut game::Game) {
        println!("TestPluginB::finish");
    }
}
fn main() {
    Game::new().add_plugin(TestPluginA).run();
}
