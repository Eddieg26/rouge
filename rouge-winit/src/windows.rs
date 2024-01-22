use rouge_ecs::{macros::LocalResource, world::resource::LocalResource};
use std::collections::HashMap;

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

    pub fn add(&mut self, id: winit::window::WindowId, window: winit::window::Window) {
        self.windows.insert(id, window);
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
