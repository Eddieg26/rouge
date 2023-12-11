use crate::{ecs::Resource, game::plugin::Plugin};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::WindowBuilder,
};

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

impl Resource for WindowEventLoop {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn name(&self) -> &str {
        "window-plugin"
    }

    fn start(&self, game: &mut crate::game::Game) {
        let events = winit::event_loop::EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("Rust Game Engine")
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
                    WindowEvent::Resized(size) => {}
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // game.resize(new_inner_size.width, new_inner_size.height)
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
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
