use super::Game;

pub trait Plugin: 'static {
    fn name(&self) -> &str;
    fn start(&self, _game: &mut Game) {}
    fn run(&self, _game: &mut Game) {}
    fn finish(&self, _game: &mut Game) {}
    fn dependencies(&self) -> Vec<Box<dyn Plugin>> {
        vec![]
    }
}
