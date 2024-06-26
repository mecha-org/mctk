//! Types that relate to user inputs.
//!
//! These are most typically interacted with through event-handling methods of [`Component`][crate::Component]. For instance [`#on_click`][crate::Component#method.on_click] receives an `Event<Click>`. A [`Click`][crate::event::Click], holds a [`MouseButton`] input type. If the user cares what kind of click they are reacting to, they need to match this input to the desired mouse button.

use std::fmt;

use crate::types::Data;

/// Mouse movement or scrolling
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Motion {
    Mouse { x: f32, y: f32 },
    Scroll { x: f32, y: f32 },
}

/// A keyboard key
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Unknown,
    Backspace,
    Tab,
    Return,
    Escape,
    Space,
    Exclaim,
    Quotedbl,
    Hash,
    Dollar,
    Percent,
    Ampersand,
    Quote,
    LeftParen,
    RightParen,
    Asterisk,
    Plus,
    Comma,
    Minus,
    Period,
    Slash,
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    Colon,
    Semicolon,
    Less,
    Equals,
    Greater,
    Question,
    At,
    LeftBracket,
    Backslash,
    RightBracket,
    Caret,
    Underscore,
    Backquote,
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
    Delete,
    CapsLock,
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
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLockClear,
    NumPadDivide,
    NumPadMultiply,
    NumPadMinus,
    NumPadPlus,
    NumPadEnter,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
    NumPad0,
    NumPadPeriod,
    Application,
    Power,
    NumPadEquals,
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
    Execute,
    Help,
    Menu,
    Select,
    Stop,
    Again,
    Undo,
    Cut,
    Copy,
    Paste,
    Find,
    Mute,
    VolumeUp,
    VolumeDown,
    NumPadComma,
    NumPadEqualsAS400,
    AltErase,
    Sysreq,
    Cancel,
    Clear,
    Prior,
    Return2,
    Separator,
    Out,
    Oper,
    ClearAgain,
    CrSel,
    ExSel,
    NumPad00,
    NumPad000,
    ThousandsSeparator,
    DecimalSeparator,
    CurrencyUnit,
    CurrencySubUnit,
    NumPadLeftParen,
    NumPadRightParen,
    NumPadLeftBrace,
    NumPadRightBrace,
    NumPadTab,
    NumPadBackspace,
    NumPadA,
    NumPadB,
    NumPadC,
    NumPadD,
    NumPadE,
    NumPadF,
    NumPadXor,
    NumPadPower,
    NumPadPercent,
    NumPadLess,
    NumPadGreater,
    NumPadAmpersand,
    NumPadDblAmpersand,
    NumPadVerticalBar,
    NumPadDblVerticalBar,
    NumPadColon,
    NumPadHash,
    NumPadSpace,
    NumPadAt,
    NumPadExclam,
    NumPadMemStore,
    NumPadMemRecall,
    NumPadMemClear,
    NumPadMemAdd,
    NumPadMemSubtract,
    NumPadMemMultiply,
    NumPadMemDivide,
    NumPadPlusMinus,
    NumPadClear,
    NumPadClearEntry,
    NumPadBinary,
    NumPadOctal,
    NumPadDecimal,
    NumPadHexadecimal,
    LCtrl,
    LShift,
    LAlt,
    LMeta,
    RCtrl,
    RShift,
    RAlt,
    RMeta,
}
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// Mouse buttons
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Aux1,
    Aux2,
}

/// Mouse or keyboard button
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Button {
    Keyboard(Key),
    Mouse(MouseButton),
}

/// Touch actions
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TouchAction {
    Up { x: f32, y: f32 },
    Down { x: f32, y: f32 },
    Cancel { x: f32, y: f32 },
    Moved { x: f32, y: f32 },
}

/// Drag and drop inputs
#[derive(Clone, Debug, PartialEq)]
pub enum Drag {
    Start(Data),
    End,
    Dragging,
    Drop(Data),
}

/// All of the inputs that lemna reacts to. Should only be needed by windows backend implementations.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Press(Button),
    Release(Button),
    Resize,
    Motion(Motion),
    Text(String),
    Focus(bool),
    Menu(i32),
    MouseLeaveWindow,
    MouseEnterWindow,
    Timer,
    Exit,
    Drag(Drag),
    Touch(TouchAction),
}
