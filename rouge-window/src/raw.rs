use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct RawWindowHandle {
    window: raw_window_handle::RawWindowHandle,
    display: raw_window_handle::RawDisplayHandle,
}

impl RawWindowHandle {
    pub fn new(
        window: raw_window_handle::RawWindowHandle,
        display: raw_window_handle::RawDisplayHandle,
    ) -> Self {
        Self { window, display }
    }

    pub fn window(&self) -> &raw_window_handle::RawWindowHandle {
        &self.window
    }

    pub fn display(&self) -> &raw_window_handle::RawDisplayHandle {
        &self.display
    }
}

unsafe impl HasRawWindowHandle for RawWindowHandle {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        self.window.clone()
    }
}

unsafe impl HasRawDisplayHandle for RawWindowHandle {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        self.display.clone()
    }
}
