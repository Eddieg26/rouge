use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rouge_ecs::{macros::LocalResource, world::resource::LocalResource};
use rouge_window::{raw::RawWindowHandle, window::WindowConfig};
use std::collections::HashMap;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::EventLoop,
};

#[derive(LocalResource)]
pub struct WinitWindows {
    windows: HashMap<winit::window::WindowId, winit::window::Window>,
    primary: Option<winit::window::WindowId>,
}

impl WinitWindows {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            primary: None,
        }
    }

    pub fn primary(&self) -> Option<&winit::window::WindowId> {
        self.primary.as_ref()
    }

    pub fn set_primary(&mut self, id: winit::window::WindowId) {
        self.primary = Some(id);
    }

    pub fn add(
        &mut self,
        event_loop: &EventLoop<()>,
        config: &WindowConfig,
    ) -> (winit::window::WindowId, rouge_window::window::Window) {
        let winit_window = winit::window::WindowBuilder::new()
            .with_title(config.title.clone())
            .with_inner_size(PhysicalSize::new(config.width, config.height))
            .with_position(PhysicalPosition::new(config.x, config.y))
            .with_decorations(config.decorated)
            .with_resizable(config.resizable)
            .with_visible(config.visible)
            .with_maximized(config.maximized)
            .with_transparent(config.transparent)
            .build(event_loop)
            .expect("Failed to create window");

        let window_handle = winit_window.raw_window_handle();
        let display_handle = winit_window.raw_display_handle();
        let handle = RawWindowHandle::new(window_handle, display_handle);
        let mut window = rouge_window::window::Window::new(
            &config.title,
            handle,
            config.mode,
            config.width,
            config.height,
        );

        window.set_focused(winit_window.has_focus());
        window.set_scale_factor(winit_window.scale_factor());
        window.set_visible(winit_window.is_visible().unwrap_or(false));
        window.set_maximized(winit_window.is_maximized());
        window.set_minimized(winit_window.is_minimized().unwrap_or(false));
        window.set_fullscreen(winit_window.fullscreen().is_some());

        let id = winit_window.id();
        self.windows.insert(id, winit_window);

        (id, window)
    }

    pub fn get(&self, id: &winit::window::WindowId) -> Option<&winit::window::Window> {
        self.windows.get(id)
    }

    pub fn get_mut(&mut self, id: &winit::window::WindowId) -> Option<&mut winit::window::Window> {
        self.windows.get_mut(id)
    }

    pub fn contains(&self, id: &winit::window::WindowId) -> bool {
        self.windows.contains_key(id)
    }

    pub fn remove(&mut self, id: &winit::window::WindowId) -> Option<winit::window::Window> {
        if self.primary == Some(*id) {
            self.primary = None;
        }

        self.windows.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &winit::window::Window> {
        self.windows.iter().map(|(_, window)| window)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut winit::window::Window> {
        self.windows.iter_mut().map(|(_, window)| window)
    }
}
