mod gl;
pub mod layer_surface;
pub mod layer_window;
pub mod lock_surface;
pub mod lock_window;
mod pointer;
mod keyboard;
mod touch;

use keyboard::KeyboardEvent;
use mctk_core::component;
use mctk_core::raw_handle::RawWaylandHandle;
use pointer::MouseEvent;
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use touch::TouchEvent;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;

pub struct PhysicalPosition<P> {
    pub x: P,
    pub y: P,
}

pub struct WindowOptions {
    pub height: u32,
    pub width: u32,
    pub scale_factor: f32,
}

#[derive(Debug)]
pub enum WindowMessage {
    Configure {
        width: u32,
        height: u32,
        wayland_handle: RawWaylandHandle,
    },
    MainEventsCleared,
    RedrawRequested,
    Send {
        message: component::Message,
    },
    WindowEvent {
        event: WindowEvent,
    },
}
#[derive(Debug, Copy, Clone)]
pub enum WindowEvent {
    CloseRequested,
    Focused,
    Unfocused,
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
    Touch(TouchEvent),
}


pub fn new_raw_wayland_handle(wl_display: &WlDisplay, wl_surface: &WlSurface) -> RawWaylandHandle {
    let wayland_handle = {
        let mut handle = WaylandDisplayHandle::empty();
        handle.display = wl_display.id().as_ptr() as *mut _;
        let display_handle = RawDisplayHandle::Wayland(handle);

        let mut handle = WaylandWindowHandle::empty();
        handle.surface = wl_surface.id().as_ptr() as *mut _;
        let window_handle = RawWindowHandle::Wayland(handle);

        RawWaylandHandle(display_handle, window_handle)
    };
    wayland_handle
}
