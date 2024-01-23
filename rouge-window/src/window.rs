use crate::raw::RawWindowHandle;
use rouge_ecs::{
    macros::LocalResource, storage::sparse::SparseMap, world::resource::LocalResource,
};
use std::hash::{Hash, Hasher};

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
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseScrollDelta {
    LineDelta(f32, f32),
    PixelDelta(f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Snapshot,
    Scroll,
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,
    Left,
    Up,
    Right,
    Down,
    Backspace,
    Return,
    Space,
    Compose,
    Caret,
    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,
    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    NavigateForward,
    NavigateBackward,
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
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
    ContextMenu,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
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
    cursor: Cursor,
}

impl Window {
    pub fn new(
        title: &str,
        handle: RawWindowHandle,
        mode: WindowMode,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            title: title.into(),
            handle,
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
            cursor: Cursor::default(),
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn handle(&self) -> &RawWindowHandle {
        &self.handle
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

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
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

    pub fn primary_id(&self) -> Option<WindowId> {
        self.primary
    }

    pub fn set_primary(&mut self, id: WindowId) {
        self.primary = Some(id);
    }

    pub fn add(&mut self, id: WindowId, window: Window) {
        self.windows.insert(id, window);
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

pub struct WindowConfig {
    pub title: String,
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub x: i32,
    pub y: i32,
    pub resizable: bool,
    pub visible: bool,
    pub decorated: bool,
    pub transparent: bool,
    pub maximized: bool,
    pub cursor: Cursor,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Rouge".into(),
            mode: WindowMode::Windowed,
            width: 800,
            height: 600,
            scale_factor: 1.0,
            x: 0,
            y: 0,
            resizable: true,
            visible: true,
            decorated: true,
            transparent: false,
            maximized: false,
            cursor: Cursor::default(),
        }
    }
}
