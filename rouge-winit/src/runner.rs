use crate::{map, EventLoopResource};
use rouge_game::game::Game;
use rouge_window::{
    actions::{
        CursorEntered, CursorLeft, CursorMoved, FileDropped, KeyboardInput, MouseWheel,
        ReceivedCharacter, WindowClosed, WindowFocused, WindowMoved, WindowResized,
        WindowScaleFactorChanged, WindowUnfocused,
    },
    window::{WindowId, Windows},
};
use winit::event::{Event, WindowEvent};

pub fn winit_runner(mut game: Game) {
    let event_loop = game
        .local_resource_mut::<EventLoopResource>()
        .take()
        .expect("EventLoopResource not found");

    let _ = event_loop.run(move |event, _, flow| match event {
        Event::WindowEvent { window_id, event } => {
            let id = WindowId::new(window_id);
            if !game.local_resource::<Windows>().contains(&id) {
                return;
            }

            let primary_id = game.local_resource::<Windows>().primary_id();
            match event {
                WindowEvent::CursorEntered { .. } => {
                    game.actions_mut().add(CursorEntered::new(id));
                    game.flush_actions::<CursorEntered>();
                }
                WindowEvent::CursorLeft { .. } => {
                    game.actions_mut().add(CursorLeft::new(id));
                    game.flush_actions::<CursorLeft>();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    game.actions_mut()
                        .add(CursorMoved::new(id, position.x, position.y));
                    game.flush_actions::<CursorMoved>();
                }
                WindowEvent::Focused(focused) => match focused {
                    true => game.actions_mut().add(WindowFocused::new(id)),
                    false => game.actions_mut().add(WindowUnfocused::new(id)),
                },
                WindowEvent::HoveredFile(path) => {
                    game.actions_mut()
                        .add(rouge_window::actions::FileHovered::new(id, path));
                    game.flush_actions::<rouge_window::actions::FileHovered>();
                }
                WindowEvent::HoveredFileCancelled => {
                    game.actions_mut()
                        .add(rouge_window::actions::FileUnhovered::new(id));
                    game.flush_actions::<rouge_window::actions::FileUnhovered>();
                }
                WindowEvent::DroppedFile(path) => {
                    game.actions_mut().add(FileDropped::new(id, path));
                    game.flush_actions::<FileDropped>();
                }
                WindowEvent::Moved(position) => {
                    game.actions_mut()
                        .add(WindowMoved::new(id, position.x, position.y));
                    game.flush_actions::<WindowMoved>();
                }
                WindowEvent::Resized(size) => {
                    game.actions_mut()
                        .add(WindowResized::new(id, size.width, size.height));
                    game.flush_actions::<WindowResized>();
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    game.actions_mut()
                        .add(WindowScaleFactorChanged::new(id, scale_factor));
                    game.flush_actions::<WindowScaleFactorChanged>();
                }
                WindowEvent::ReceivedCharacter(character) => {
                    game.actions_mut()
                        .add(ReceivedCharacter::new(id, character));
                    game.flush_actions::<ReceivedCharacter>();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let delta = map::map_mouse_scroll_delta(delta);
                    game.actions_mut().add(MouseWheel::new(id, delta));
                    game.flush_actions::<MouseWheel>();
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    let state = map::map_key_state(input.state);
                    let code = input.virtual_keycode.map(map::map_keycode);

                    game.actions_mut().add(KeyboardInput::new(id, state, code));
                    game.flush_actions::<KeyboardInput>();
                }
                WindowEvent::CloseRequested => {
                    game.actions_mut().add(WindowClosed::new(id));
                    game.flush_actions::<WindowClosed>();
                    match primary_id {
                        Some(primary_id) if primary_id == id => flow.set_exit(),
                        _ => flow.set_exit(),
                    }
                }
                WindowEvent::Destroyed => {
                    game.actions_mut()
                        .add(rouge_window::actions::WindowDestroyed::new(id));
                    match primary_id {
                        Some(primary_id) if primary_id == id => flow.set_exit(),
                        _ => flow.set_exit(),
                    }
                }
                _ => {}
            }
        }
        Event::MainEventsCleared => {
            game.update();
        }
        _ => {}
    });
}
