use wayland_client::protocol::wl_surface::WlSurface;

#[derive(Debug, Clone)]
pub struct TouchPoint {
    pub surface: WlSurface,
    pub position: Position,
}

#[derive(Debug, Copy, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Copy, Clone)]
pub enum TouchEvent {
    Up { id: i32, time: u32, position: Position, scale_factor: f32 },
    Down { id: i32, time: u32, position: Position, scale_factor: f32 },
    Motion { id: i32, time: u32, position: Position, scale_factor: f32 },
    Cancel { id: i32, position: Position, scale_factor: f32},
}
