use std::hash::{Hash, Hasher};

pub use events::*;

pub trait Event: 'static {}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventType(u64);

impl EventType {
    pub fn new<E: Event>() -> EventType {
        let type_id = std::any::TypeId::of::<E>();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        EventType(hasher.finish())
    }
}

pub struct EventListeners<E: Event> {
    listeners: Vec<Box<dyn Fn(&E) + Send + Sync>>,
}

impl<E: Event> EventListeners<E> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn add_listener<F: Fn(&E) + Send + Sync + 'static>(&mut self, listener: F) {
        self.listeners.push(Box::new(listener));
    }

    pub fn invoke(&self, event: &E) {
        for listener in &self.listeners {
            listener(event);
        }
    }
}

pub trait BaseListeners: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<E: Event> BaseListeners for EventListeners<E> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub mod events {
    use rouge_window::window::KeyCode;

    use super::Event;

    pub struct Click;

    impl Event for Click {}

    pub struct MouseMove {
        pub x: f32,
        pub y: f32,
    }

    impl Event for MouseMove {}

    pub struct Hover;

    impl Event for Hover {}

    pub struct UnHover;

    impl Event for UnHover {}

    pub struct Focus;

    impl Event for Focus {}

    pub struct UnFocus;

    impl Event for UnFocus {}

    pub struct Keydown {
        pub code: KeyCode,
    }

    impl Event for Keydown {}

    pub struct Keyup {
        pub code: KeyCode,
    }

    impl Event for Keyup {}

    pub struct KeyPress {
        pub code: KeyCode,
    }
}
