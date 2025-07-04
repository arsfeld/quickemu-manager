use super::Result;

/// Input event that can be either keyboard or mouse
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
}

/// Keyboard event with scancode and optional keycode
#[derive(Debug, Clone, Copy)]
pub enum KeyboardEvent {
    KeyDown {
        scancode: u32,
        keycode: Option<u32>,
        modifiers: u32,
    },
    KeyUp {
        scancode: u32,
        keycode: Option<u32>,
        modifiers: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
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
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
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
    Escape,
    Tab,
    CapsLock,
    Shift,
    Ctrl,
    Alt,
    Super,
    Space,
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    PrintScreen,
    ScrollLock,
    Pause,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Motion {
        x: u32,
        y: u32,
        relative_x: i32,
        relative_y: i32,
    },
    Button {
        button: MouseButton,
        pressed: bool,
        x: u32,
        y: u32,
    },
    Wheel {
        delta_x: i32,
        delta_y: i32,
    },
}

/// Legacy keyboard event struct for InputHandler trait
#[derive(Debug, Clone, Copy)]
pub struct LegacyKeyboardEvent {
    pub key: KeyCode,
    pub pressed: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

pub trait InputHandler {
    fn handle_keyboard(&mut self, event: LegacyKeyboardEvent) -> Result<()>;

    fn handle_mouse(&mut self, event: MouseEvent) -> Result<()>;

    fn grab_input(&mut self, grab: bool) -> Result<()>;

    fn is_grabbed(&self) -> bool;

    fn set_relative_mouse(&mut self, relative: bool) -> Result<()>;

    fn warp_mouse(&mut self, x: i32, y: i32) -> Result<()>;
}
