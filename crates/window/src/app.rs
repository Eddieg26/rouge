use crate::{
    events::{
        AxisMotion, CursorEntered, CursorLeft, CursorMoved, DoubleTapGesture, DroppedFile,
        HoveredFile, HoveredFileCancelled, KeyEvent, ModifiersChanged, MouseInput, MouseScroll,
        PanGesture, PinchGesture, RotationGesture, ScaleFactorChanged, TouchEvent,
        TouchpadPressure, WindowClosed, WindowCreated, WindowDestroyed, WindowFocused, WindowMoved,
        WindowOccluded, WindowResized,
    },
    window::{Window, WindowConfig},
};
use ecs::{
    event::{Event, Events},
    world::action::WorldAction,
};
use game::{ExitGame, Game};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

pub struct App {
    game: Game,
}

impl App {
    pub fn new(game: Game) -> Self {
        Self { game }
    }

    pub fn start(&mut self) {
        self.game.startup();
    }

    pub fn update(&mut self) -> Option<ExitGame> {
        self.game.update()
    }

    pub fn shutdown(&mut self) {
        self.game.shutdown();
    }

    fn run_event<E: Event>(&mut self, event: E) {
        self.game
            .app_mut()
            .world_mut()
            .resource_mut::<Events<E>>()
            .add(event);
        self.game.flush_type::<E>();
    }

    fn run(&mut self, event_loop: EventLoop<()>) {
        event_loop.set_control_flow(ControlFlow::Poll);

        if let Err(e) = event_loop.run_app(self) {
            let error = AppRunError::new(e);
            let actions = self.game.app_mut().world_mut().actions();
            actions.add(error);
        }

        self.shutdown();
    }

    pub fn runner(mut game: Game) {
        match EventLoop::new() {
            Ok(event_loop) => App::new(game).run(event_loop),
            Err(e) => {
                let error = AppRunError::new(e);
                let actions = game.app_mut().world_mut().actions();
                actions.add(error);
                game.flush();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let world = self.game.app_mut().world_mut();
        let has_window = world.try_non_send_resource::<Window>().is_some();
        match (has_window, world.remove_resource::<WindowConfig>()) {
            (false, Some(config)) => {
                let window = Window::new(config, event_loop);
                let id = window.id();
                world.add_non_send_resource(window);
                self.run_event(WindowCreated::new(id));
                self.start();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(exit) = self.update() {
            println!("Exiting: {:?}", exit);
            self.run_event(exit);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.run_event(WindowClosed::new(window));
                event_loop.exit();
            }
            WindowEvent::Destroyed => self.run_event(WindowDestroyed::new(window)),
            WindowEvent::Resized(size) => self.run_event(WindowResized::new(size)),
            WindowEvent::Moved(position) => self.run_event(WindowMoved::new(position)),
            WindowEvent::DroppedFile(path) => self.run_event(DroppedFile::new(path)),
            WindowEvent::HoveredFile(path) => self.run_event(HoveredFile::new(path)),
            WindowEvent::HoveredFileCancelled => self.run_event(HoveredFileCancelled),
            WindowEvent::Focused(focused) => self.run_event(WindowFocused::new(focused)),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => self.run_event(KeyEvent::new(device_id, event, is_synthetic)),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.run_event(ModifiersChanged::new(modifiers))
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => self.run_event(CursorMoved::new(device_id, position)),
            WindowEvent::CursorEntered { device_id } => {
                self.run_event(CursorEntered::new(device_id))
            }
            WindowEvent::CursorLeft { device_id } => self.run_event(CursorLeft::new(device_id)),
            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => self.run_event(AxisMotion::new(device_id, axis, value)),
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => self.run_event(MouseScroll::new(device_id, delta, phase)),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => self.run_event(MouseInput::new(device_id, state, button)),
            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(PinchGesture::new(device_id, delta, phase)),
            WindowEvent::PanGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(PanGesture::new(device_id, delta, phase)),
            WindowEvent::DoubleTapGesture { device_id } => {
                self.run_event(DoubleTapGesture::new(device_id))
            }
            WindowEvent::RotationGesture {
                device_id,
                delta,
                phase,
            } => self.run_event(RotationGesture::new(device_id, delta, phase)),
            WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => self.run_event(TouchpadPressure::new(device_id, pressure, stage)),
            WindowEvent::Touch(touch) => self.run_event(TouchEvent::from(touch)),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.run_event(ScaleFactorChanged::new(scale_factor))
            }
            WindowEvent::Occluded(occluded) => self.run_event(WindowOccluded::new(occluded)),
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct AppRunError(EventLoopError);

impl AppRunError {
    fn new(error: EventLoopError) -> Self {
        Self(error)
    }

    pub fn error(&self) -> &EventLoopError {
        &self.0
    }
}

impl std::ops::Deref for AppRunError {
    type Target = EventLoopError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for AppRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for AppRunError {}

impl WorldAction for AppRunError {
    fn execute(self, world: &mut ecs::world::World) -> Option<()> {
        world
            .resource_mut::<Events<ExitGame>>()
            .add(ExitGame::failure(self));
        Some(())
    }
}
