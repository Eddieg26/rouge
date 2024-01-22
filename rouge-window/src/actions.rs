use crate::window::WindowId;
use rouge_ecs::system::observer::Action;
use std::path::PathBuf;

pub struct WindowCreated {
    pub id: WindowId,
}

impl WindowCreated {
    pub fn new(id: WindowId) -> Self {
        Self { id }
    }
}

impl Action for WindowCreated {
    type Output = WindowId;

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        self.id
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowResized {
    pub id: WindowId,
    pub width: u32,
    pub height: u32,
}

impl WindowResized {
    pub fn new(id: WindowId, width: u32, height: u32) -> Self {
        Self { id, width, height }
    }
}

impl Action for WindowResized {
    type Output = Self;

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        todo!()
    }
}

pub struct WindowMoved {
    pub id: WindowId,
    pub x: i32,
    pub y: i32,
}

pub struct WindowClosed {
    pub id: WindowId,
}

pub struct WindowFocused {
    pub id: WindowId,
}

pub struct WindowUnfocused {
    pub id: WindowId,
}

pub struct WindowRefreshed {
    pub id: WindowId,
}

pub struct WindowMinimized {
    pub id: WindowId,
}

pub struct WindowMaximized {
    pub id: WindowId,
}

pub struct WindowRestored {
    pub id: WindowId,
}

pub struct WindowHovered {
    pub id: WindowId,
}

pub struct WindowUnhovered {
    pub id: WindowId,
}

pub struct WindowScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}

pub struct CursorEntered {
    pub id: WindowId,
}

pub struct CursorLeft {
    pub id: WindowId,
}

pub struct CursorMoved {
    pub id: WindowId,
    pub x: f64,
    pub y: f64,
}

pub struct FileHovered {
    pub id: WindowId,
    pub path: PathBuf,
}

pub struct FileUnhovered {
    pub id: WindowId,
    pub path: PathBuf,
}

pub struct FileDropped {
    pub id: WindowId,
    pub path: PathBuf,
}
