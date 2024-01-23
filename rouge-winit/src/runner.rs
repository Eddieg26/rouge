use crate::{map, EventLoopResource};
use rouge_ecs::system::observer::Actions;
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
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(CursorEntered::new(id));
                    // game.flush_actions::<CursorEntered>();
                }
                WindowEvent::CursorLeft { .. } => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(CursorLeft::new(id));
                    // game.flush_actions::<CursorLeft>();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(CursorMoved::new(id, position.x, position.y));
                    // game.flush_actions::<CursorMoved>();
                }
                WindowEvent::Focused(focused) => {
                    // let actions = game.resource_mut::<Actions>();
                    // match focused {
                    //     true => actions.add(WindowFocused::new(id)),
                    //     false => actions.add(WindowUnfocused::new(id)),
                    // }
                }
                WindowEvent::HoveredFile(path) => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(rouge_window::actions::FileHovered::new(id, path));
                    // game.flush_actions::<rouge_window::actions::FileHovered>();
                }
                WindowEvent::HoveredFileCancelled => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(rouge_window::actions::FileUnhovered::new(id));
                    // game.flush_actions::<rouge_window::actions::FileUnhovered>();
                }
                WindowEvent::DroppedFile(path) => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(FileDropped::new(id, path));
                    // game.flush_actions::<FileDropped>();
                }
                WindowEvent::Moved(position) => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(WindowMoved::new(id, position.x, position.y));
                    // game.flush_actions::<WindowMoved>();
                }
                WindowEvent::Resized(size) => {
                    let actions = game.resource_mut::<Actions>();
                    actions.add(WindowResized::new(id, size.width, size.height));
                    game.flush_actions::<WindowResized>();
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(WindowScaleFactorChanged::new(id, scale_factor));
                    // game.flush_actions::<WindowScaleFactorChanged>();
                }
                WindowEvent::ReceivedCharacter(character) => {
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(ReceivedCharacter::new(id, character));
                    // game.flush_actions::<ReceivedCharacter>();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // let actions = game.resource_mut::<Actions>();
                    // let delta = map::map_mouse_scroll_delta(delta);
                    // actions.add(MouseWheel::new(id, delta));
                    // game.flush_actions::<MouseWheel>();
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    // let state = map::map_key_state(input.state);
                    // let code = input.virtual_keycode.map(map::map_keycode);
                    // let actions = game.resource_mut::<Actions>();
                    // actions.add(KeyboardInput::new(id, state, code));
                    // game.flush_actions::<KeyboardInput>();
                }
                WindowEvent::CloseRequested => {
                    let actions = game.resource_mut::<Actions>();
                    actions.add(WindowClosed::new(id));
                    game.flush_actions::<WindowClosed>();
                }
                WindowEvent::Destroyed => {
                    let actions = game.resource_mut::<Actions>();
                    actions.add(rouge_window::actions::WindowDestroyed::new(id));
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
