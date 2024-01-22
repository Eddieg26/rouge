use rouge_ecs::{
    macros::LocalResource, storage::sparse::SparseMap, world::resource::LocalResource,
};
use std::hash::{Hash, Hasher};

use crate::raw::RawWindowHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(u64);

impl WindowId {
    pub fn new(id: impl Hash) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

impl std::ops::Deref for WindowId {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for WindowId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowMode {
    Windowed,
    Fullscreen,
    Borderless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentMode {
    Immediate,
    VSync,
    Mailbox,
    Fifo,
    FifoRelaxed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositeAlphaMode {
    Auto,
    Opaque,
    PreMultiplied,
    PostMultiplied,
    Inherit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorIcon {
    Default,
    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    Progress,
    NotAllowed,
    ResizeHorizontal,
    ResizeVertical,
    ResizeTopLeftBottomRight,
    ResizeTopRightBottomLeft,
    ResizeAll,
    NoDrop,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    ZoomIn,
    ZoomOut,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    pub icon: CursorIcon,
    pub visible: bool,
    pub x: f64,
    pub y: f64,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            icon: CursorIcon::Default,
            visible: true,
            x: 0.0,
            y: 0.0,
        }
    }
}

pub struct Window {
    title: String,
    handle: RawWindowHandle,
    present_mode: PresentMode,
    composite_alpha_mode: CompositeAlphaMode,
    mode: WindowMode,
    width: u32,
    height: u32,
    scale_factor: f64,
    x: i32,
    y: i32,
    fullscreen: bool,
    resizable: bool,
    visible: bool,
    focused: bool,
    hovered: bool,
    decorated: bool,
    transparent: bool,
    maximized: bool,
    minimized: bool,
    always_on_top: bool,
    cursor: Cursor,
}

impl Window {
    pub fn new(
        title: &str,
        handle: RawWindowHandle,
        present_mode: PresentMode,
        composite_alpha_mode: CompositeAlphaMode,
        mode: WindowMode,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            title: title.into(),
            handle,
            present_mode,
            composite_alpha_mode,
            mode,
            width,
            height,
            scale_factor: 1.0,
            x: 0,
            y: 0,
            fullscreen: false,
            resizable: true,
            visible: true,
            focused: true,
            hovered: false,
            decorated: true,
            transparent: false,
            maximized: false,
            minimized: false,
            always_on_top: false,
            cursor: Cursor::default(),
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn handle(&self) -> &RawWindowHandle {
        &self.handle
    }

    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    pub fn composite_alpha_mode(&self) -> CompositeAlphaMode {
        self.composite_alpha_mode
    }

    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn resizable(&self) -> bool {
        self.resizable
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn hovered(&self) -> bool {
        self.hovered
    }

    pub fn decorated(&self) -> bool {
        self.decorated
    }

    pub fn transparent(&self) -> bool {
        self.transparent
    }

    pub fn maximized(&self) -> bool {
        self.maximized
    }

    pub fn minimized(&self) -> bool {
        self.minimized
    }

    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_present_mode(&mut self, present_mode: PresentMode) {
        self.present_mode = present_mode;
    }

    pub fn set_composite_alpha_mode(&mut self, composite_alpha_mode: CompositeAlphaMode) {
        self.composite_alpha_mode = composite_alpha_mode;
    }

    pub fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        self.hovered = hovered;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn set_decorated(&mut self, decorated: bool) {
        self.decorated = decorated;
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        self.maximized = maximized;
        if self.maximized {
            self.minimized = false;
        }
    }

    pub fn set_minimized(&mut self, minimized: bool) {
        self.minimized = minimized;
        if self.minimized {
            self.maximized = false;
        }
    }

    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.always_on_top = always_on_top;
    }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    pub fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.cursor.icon = icon;
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor.visible = visible;
    }

    pub fn set_cursor_position(&mut self, x: f64, y: f64) {
        self.cursor.x = x;
        self.cursor.y = y;
    }
}

#[derive(LocalResource)]
pub struct Windows {
    windows: SparseMap<WindowId, Window>,
    primary: Option<WindowId>,
}

impl Windows {
    pub fn new() -> Self {
        Self {
            windows: SparseMap::new(),
            primary: None,
        }
    }

    pub fn primary(&self) -> Option<&Window> {
        self.primary.and_then(|id| self.windows.get(&id))
    }

    pub fn set_primary(&mut self, id: WindowId) {
        self.primary = Some(id);
    }

    pub fn add(&mut self, id: impl Hash, window: Window) {
        self.windows.insert(WindowId::new(id), window);
    }

    pub fn get(&self, id: &WindowId) -> Option<&Window> {
        self.windows.get(id)
    }

    pub fn get_mut(&mut self, id: &WindowId) -> Option<&mut Window> {
        self.windows.get_mut(id)
    }

    pub fn contains(&self, id: &WindowId) -> bool {
        self.windows.contains(id)
    }

    pub fn remove(&mut self, id: &WindowId) -> Option<Window> {
        if self.primary == Some(*id) {
            self.primary = None;
        }

        self.windows.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.iter().map(|(_, window)| window)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.windows.iter_mut().map(|(_, window)| window)
    }
}
