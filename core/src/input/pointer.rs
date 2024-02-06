
#[derive(Debug, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub enum ScrollDelta {
    Lines { x: f32, y: f32 },
    Pixels { x: f32, y: f32 }
}

#[derive(Debug, Clone)]
pub enum MouseEvent {
    CursorEntered,
    CursorLeft,
    CursorMoved { position: Point },
    ButtonPressed { button: Button },
    ButtonReleased{ button: Button },
    WheelScrolled { delta: ScrollDelta },
}

#[derive(Debug, Clone)]
pub enum Cursor {
    Available { position: Point },
    Unavailable,
}

pub fn convert_button(button: u32) -> Option<Button> {
    match button {
        0x110 => Some(Button::Left),
        0x111 => Some(Button::Right),
        0x112 => Some(Button::Middle),
        _ => None,
    }
}
