use rouge_ecs::{macros::LocalResource, world::resource::LocalResource};
use rouge_game::{
    game::Game,
    plugin::{Plugin, Plugins},
};
use rouge_window::{
    window::{WindowConfig, WindowId, Windows},
    WindowPlugin,
};
use windows::WinitWindows;
use winit::event_loop::EventLoop;

pub mod map;
pub mod runner;
pub mod systems;
pub mod windows;

#[derive(LocalResource)]
pub struct EventLoopResource(Option<EventLoop<()>>);

impl std::ops::Deref for EventLoopResource {
    type Target = Option<EventLoop<()>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EventLoopResource {
    pub fn new(event_loop: EventLoop<()>) -> Self {
        Self(Some(event_loop))
    }

    pub fn get(&self) -> Option<&EventLoop<()>> {
        self.0.as_ref()
    }

    pub fn take(&mut self) -> Option<EventLoop<()>> {
        self.0.take()
    }
}

pub struct WinitPlugin {
    config: WindowConfig,
}

impl WinitPlugin {
    pub fn new() -> Self {
        Self {
            config: WindowConfig::default(),
        }
    }

    pub fn with_config(config: WindowConfig) -> Self {
        Self { config }
    }
}

impl Plugin for WinitPlugin {
    fn plugins(&self, plugins: &mut Plugins) {
        plugins.register(WindowPlugin)
    }

    fn start(&mut self, game: &mut Game) {
        let event_loop = EventLoop::new();
        let event_loop = EventLoopResource::new(event_loop);

        game.add_local_resource(event_loop);
        game.add_local_resource(WinitWindows::new());
    }

    fn run(&mut self, game: &mut Game) {
        let event_loop = game
            .local_resource_mut::<EventLoopResource>()
            .get()
            .unwrap();
        let winit_windows = game.local_resource_mut::<WinitWindows>();
        let (id, window) = winit_windows.add(event_loop, &self.config);

        let windows = game.local_resource_mut::<Windows>();
        windows.add(WindowId::new(id), window);
    }

    fn finish(&mut self, game: &mut Game) {
        game.with_runner(runner::winit_runner);
    }
}
