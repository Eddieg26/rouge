use ecs::Resource;
use game::{plugin::Plugin, Game};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub mod ecs;
pub mod game;
pub mod graphics;
pub mod primitives;
pub mod tree;

fn main() {
    println!("Hello, world!");
}

pub struct WinitPlugin;

impl Plugin for WinitPlugin {
    fn name(&self) -> &str {
        "winit-plugin"
    }

    fn start(&self, game: &mut game::Game) {
        todo!()
    }

    fn run(&self, game: &mut game::Game) {
        todo!()
    }

    fn finish(&self, game: &mut game::Game) {
        game.with_runner(runner);
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

fn runner(mut game: Game) {
    let events = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Rust Game Engine")
        .build(&events)
        .unwrap();

    let id = window.id();
    game.add_resource(window);

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
}
