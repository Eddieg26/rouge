use crate::window::{Window, WindowId, Windows};
use rouge_ecs::system::observer::Action;
use std::path::PathBuf;

pub struct WindowCreated {
    id: WindowId,
    window: Option<Window>,
}

impl WindowCreated {
    pub fn new(id: WindowId, window: Window) -> Self {
        Self {
            id,
            window: Some(window),
        }
    }
}

impl Action for WindowCreated {
    type Output = WindowId;

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        world
            .local_resource_mut::<Windows>()
            .add(self.id, self.window.take().unwrap());
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

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        self.width == 0 || self.height == 0 || !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_size(self.width, self.height);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowMoved {
    pub id: WindowId,
    pub x: i32,
    pub y: i32,
}

impl WindowMoved {
    pub fn new(id: WindowId, x: i32, y: i32) -> Self {
        Self { id, x, y }
    }
}

impl Action for WindowMoved {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_position(self.x, self.y);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowClosed {
    pub id: WindowId,
}

impl Action for WindowClosed {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        world
            .local_resource_mut::<Windows>()
            .remove(&self.id)
            .unwrap();
        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowFocused {
    pub id: WindowId,
}

impl Action for WindowFocused {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_focused(true);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowUnfocused {
    pub id: WindowId,
}

impl Action for WindowUnfocused {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_focused(false);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowRefreshed {
    pub id: WindowId,
}

impl Action for WindowRefreshed {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowMinimized {
    pub id: WindowId,
}

impl Action for WindowMinimized {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_minimized(true);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowMaximized {
    pub id: WindowId,
}

impl Action for WindowMaximized {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_maximized(true);

        *self
    }
}

#[derive(Clone, Copy, Debug)]

pub struct WindowRestored {
    pub id: WindowId,
}

impl Action for WindowRestored {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();
        window.set_maximized(false);
        window.set_minimized(false);

        *self
    }
}

#[derive(Clone, Copy, Debug)]

pub struct WindowHovered {
    pub id: WindowId,
}

impl Action for WindowHovered {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_hovered(true);

        *self
    }
}

#[derive(Clone, Copy, Debug)]

pub struct WindowUnhovered {
    pub id: WindowId,
}

impl Action for WindowUnhovered {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_hovered(false);

        *self
    }
}

#[derive(Clone, Copy, Debug)]

pub struct WindowScaleFactorChanged {
    pub id: WindowId,
    pub scale_factor: f64,
}

impl Action for WindowScaleFactorChanged {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_scale_factor(self.scale_factor);

        *self
    }
}

#[derive(Clone, Copy, Debug)]

pub struct CursorEntered {
    pub id: WindowId,
}

impl Action for CursorEntered {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_cursor_visible(true);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CursorLeft {
    pub id: WindowId,
}

impl Action for CursorLeft {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_cursor_visible(false);

        *self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CursorMoved {
    pub id: WindowId,
    pub x: f64,
    pub y: f64,
}

impl Action for CursorMoved {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        self.x == 0.0 || self.y == 0.0 || !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, world: &mut rouge_ecs::world::World) -> Self::Output {
        let window = world
            .local_resource_mut::<Windows>()
            .get_mut(&self.id)
            .unwrap();

        window.set_cursor_position(self.x, self.y);

        *self
    }
}

#[derive(Clone, Debug)]
pub struct FileHovered {
    pub id: WindowId,
    pub path: PathBuf,
}

impl Action for FileHovered {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        self.path.as_os_str().is_empty() || !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        self.clone()
    }
}

#[derive(Clone, Debug)]
pub struct FileUnhovered {
    pub id: WindowId,
    pub path: PathBuf,
}

impl Action for FileUnhovered {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        self.path.as_os_str().is_empty() || !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        self.clone()
    }
}

#[derive(Clone, Debug)]
pub struct FileDropped {
    pub id: WindowId,
    pub path: PathBuf,
}

impl Action for FileDropped {
    type Output = Self;

    fn skip(&self, world: &rouge_ecs::world::World) -> bool {
        self.path.as_os_str().is_empty() || !world.local_resource::<Windows>().contains(&self.id)
    }

    fn execute(&mut self, _: &mut rouge_ecs::world::World) -> Self::Output {
        self.clone()
    }
}
