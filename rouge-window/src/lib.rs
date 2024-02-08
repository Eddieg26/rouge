use rouge_game::{game::Game, plugin::Plugin};
use window::Windows;

pub mod actions;
pub mod raw;
pub mod window;

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn start(&mut self, game: &mut Game) {
        game.add_local_resource(Windows::new())
            .register_action::<actions::WindowCreated>()
            .register_action::<actions::WindowResized>()
            .register_action::<actions::WindowMoved>()
            .register_action::<actions::WindowFocused>()
            .register_action::<actions::WindowUnfocused>()
            .register_action::<actions::WindowClosed>()
            .register_action::<actions::WindowDestroyed>()
            .register_action::<actions::WindowRefreshed>()
            .register_action::<actions::WindowMinimized>()
            .register_action::<actions::WindowMaximized>()
            .register_action::<actions::WindowRestored>()
            .register_action::<actions::WindowHovered>()
            .register_action::<actions::WindowUnhovered>()
            .register_action::<actions::WindowScaleFactorChanged>()
            .register_action::<actions::CursorEntered>()
            .register_action::<actions::CursorLeft>()
            .register_action::<actions::CursorMoved>()
            .register_action::<actions::KeyboardInput>()
            .register_action::<actions::MouseWheel>()
            .register_action::<actions::FileDropped>()
            .register_action::<actions::FileUnhovered>()
            .register_action::<actions::FileHovered>()
            .register_action::<actions::ReceivedCharacter>();
    }
}
