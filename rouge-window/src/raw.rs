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

impl raw_window_handle::HasWindowHandle for RawWindowHandle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.window.clone(),
            ))
        }
    }
}

impl raw_window_handle::HasDisplayHandle for RawWindowHandle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        unsafe {
            Ok(raw_window_handle::DisplayHandle::borrow_raw(
                self.display.clone(),
            ))
        }
    }
}
