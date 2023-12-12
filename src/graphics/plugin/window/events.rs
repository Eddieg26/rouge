use crate::ecs::world::Event;

pub struct WindowResized {
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl WindowResized {
    pub fn new(size: winit::dpi::PhysicalSize<u32>) -> WindowResized {
        WindowResized { size }
    }
}

impl Event for WindowResized {
    type Data = Self;

    fn execute(&mut self, _: &crate::ecs::World) -> Self::Data {
        WindowResized { size: self.size }
    }
}

pub struct WindowMoved {
    pub x: i32,
    pub y: i32,
}

impl WindowMoved {
    pub fn new(x: i32, y: i32) -> WindowMoved {
        WindowMoved { x, y }
    }
}

impl Event for WindowMoved {
    type Data = Self;

    fn execute(&mut self, _: &crate::ecs::World) -> Self::Data {
        WindowMoved {
            x: self.x,
            y: self.y,
        }
    }
}

pub struct WindowClosed;

impl Event for WindowClosed {
    type Data = Self;

    fn execute(&mut self, _: &crate::ecs::World) -> Self::Data {
        WindowClosed
    }
}

pub struct WindowFocused;

impl Event for WindowFocused {
    type Data = Self;

    fn execute(&mut self, _: &crate::ecs::World) -> Self::Data {
        WindowFocused
    }
}

pub struct WindowUnfocused;

impl Event for WindowUnfocused {
    type Data = Self;

    fn execute(&mut self, _: &crate::ecs::World) -> Self::Data {
        WindowUnfocused
    }
}
