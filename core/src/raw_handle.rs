use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

#[derive(Debug, Clone, Copy)]
pub struct RawWaylandHandle(pub RawDisplayHandle, pub RawWindowHandle);

unsafe impl HasRawDisplayHandle for RawWaylandHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.0
    }
}

unsafe impl HasRawWindowHandle for RawWaylandHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.1
    }
}

// This is safe because for wayland we can pass handles between threads
// ref: https://github.com/rust-windowing/raw-window-handle/issues/85
unsafe impl Send for RawWaylandHandle {}
unsafe impl Sync for RawWaylandHandle {}
