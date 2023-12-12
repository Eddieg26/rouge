use crate::{ecs::Resource, game::plugin::Plugin};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::{Window, WindowBuilder},
};

use self::events::{WindowClosed, WindowFocused, WindowMoved, WindowResized, WindowUnfocused};

pub mod events;

pub struct WindowEventLoop {
    pub event_loop: Option<winit::event_loop::EventLoop<()>>,
}

impl WindowEventLoop {
    pub fn new(event_loop: winit::event_loop::EventLoop<()>) -> WindowEventLoop {
        WindowEventLoop {
            event_loop: Some(event_loop),
        }
    }
}

impl Resource for Window {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Resource for WindowEventLoop {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl WindowConfig {
    pub fn new(title: &str, width: u32, height: u32) -> WindowConfig {
        WindowConfig {
            title: title.to_string(),
            width,
            height,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "Rouge Game Engine".to_string(),
            width: 800,
            height: 600,
        }
    }
}

pub struct WindowPlugin {
    config: WindowConfig,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            config: WindowConfig::default(),
        }
    }
}

impl WindowPlugin {
    pub fn new(config: WindowConfig) -> WindowPlugin {
        WindowPlugin { config }
    }
}

impl Plugin for WindowPlugin {
    fn name(&self) -> &str {
        "window-plugin"
    }

    fn start(&self, game: &mut crate::game::Game) {
        let events = winit::event_loop::EventLoop::new();

        let window = WindowBuilder::new()
            .with_title(self.config.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ))
            .build(&events)
            .unwrap();

        game.add_resource(WindowEventLoop::new(events));
        game.add_resource(window);
    }

    fn finish(&self, game: &mut crate::game::Game) {
        let runner = |mut game: crate::game::Game| {
            let events = {
                let mut events = game.world().resource_mut::<WindowEventLoop>();
                events.event_loop.take().unwrap()
            };
            let id = game.world().resource::<winit::window::Window>().id();

            let _ = events.run(move |event, _, flow| match event {
                Event::WindowEvent { window_id, event } if window_id == id => match event {
                    WindowEvent::Resized(size) => {
                        game.world()
                            .add_event(WindowResized::new(size))
                            .dispatch_type::<WindowResized>();
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        game.world()
                            .add_event(WindowResized::new(*new_inner_size))
                            .dispatch_type::<WindowResized>();
                    }
                    WindowEvent::Moved(position) => {
                        game.world()
                            .add_event(events::WindowMoved::new(position.x, position.y))
                            .dispatch_type::<WindowMoved>();
                    }
                    WindowEvent::Focused(value) => match value {
                        true => {
                            game.world()
                                .add_event(events::WindowFocused)
                                .dispatch_type::<WindowFocused>();
                        }
                        false => {
                            game.world()
                                .add_event(events::WindowUnfocused)
                                .dispatch_type::<WindowUnfocused>();
                        }
                    },
                    WindowEvent::CloseRequested => {
                        game.world()
                            .add_event(events::WindowClosed)
                            .dispatch_type::<WindowClosed>();

                        flow.set_exit();
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => flow.set_exit(),
                    _ => {}
                },
                Event::MainEventsCleared => {
                    if game.update().is_none() {
                        flow.set_exit();
                    }
                }
                _ => {}
            });
        };

        game.with_runner(runner);
    }
}
