
#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Copy, Clone)]
pub enum Cursor {
    Available { position: Point },
    Unavailable,
}
