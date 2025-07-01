use crate::multimedia::{
    input::{InputHandler, LegacyKeyboardEvent, MouseEvent, KeyCode, MouseButton},
    Result,
};
use gtk4::{gdk, glib};
use std::sync::{Arc, Mutex};

pub struct Gtk4Input {
    grabbed: Arc<Mutex<bool>>,
    relative_mode: Arc<Mutex<bool>>,
}

// Safety: GTK4 input is designed to be accessed from the main thread
unsafe impl Send for Gtk4Input {}

impl Gtk4Input {
    pub fn new() -> Result<Self> {
        Ok(Self {
            grabbed: Arc::new(Mutex::new(false)),
            relative_mode: Arc::new(Mutex::new(false)),
        })
    }

    /// Convert GDK keyval to our KeyCode enum
    pub fn keyval_to_keycode(keyval: gdk::Key) -> KeyCode {
        use gdk::Key;
        
        match keyval {
            Key::a | Key::A => KeyCode::A,
            Key::b | Key::B => KeyCode::B,
            Key::c | Key::C => KeyCode::C,
            Key::d | Key::D => KeyCode::D,
            Key::e | Key::E => KeyCode::E,
            Key::f | Key::F => KeyCode::F,
            Key::g | Key::G => KeyCode::G,
            Key::h | Key::H => KeyCode::H,
            Key::i | Key::I => KeyCode::I,
            Key::j | Key::J => KeyCode::J,
            Key::k | Key::K => KeyCode::K,
            Key::l | Key::L => KeyCode::L,
            Key::m | Key::M => KeyCode::M,
            Key::n | Key::N => KeyCode::N,
            Key::o | Key::O => KeyCode::O,
            Key::p | Key::P => KeyCode::P,
            Key::q | Key::Q => KeyCode::Q,
            Key::r | Key::R => KeyCode::R,
            Key::s | Key::S => KeyCode::S,
            Key::t | Key::T => KeyCode::T,
            Key::u | Key::U => KeyCode::U,
            Key::v | Key::V => KeyCode::V,
            Key::w | Key::W => KeyCode::W,
            Key::x | Key::X => KeyCode::X,
            Key::y | Key::Y => KeyCode::Y,
            Key::z | Key::Z => KeyCode::Z,
            
            Key::_0 | Key::parenright => KeyCode::Num0,
            Key::_1 | Key::exclam => KeyCode::Num1,
            Key::_2 | Key::at => KeyCode::Num2,
            Key::_3 | Key::numbersign => KeyCode::Num3,
            Key::_4 | Key::dollar => KeyCode::Num4,
            Key::_5 | Key::percent => KeyCode::Num5,
            Key::_6 | Key::asciicircum => KeyCode::Num6,
            Key::_7 | Key::ampersand => KeyCode::Num7,
            Key::_8 | Key::asterisk => KeyCode::Num8,
            Key::_9 | Key::parenleft => KeyCode::Num9,
            
            Key::F1 => KeyCode::F1,
            Key::F2 => KeyCode::F2,
            Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4,
            Key::F5 => KeyCode::F5,
            Key::F6 => KeyCode::F6,
            Key::F7 => KeyCode::F7,
            Key::F8 => KeyCode::F8,
            Key::F9 => KeyCode::F9,
            Key::F10 => KeyCode::F10,
            Key::F11 => KeyCode::F11,
            Key::F12 => KeyCode::F12,
            
            Key::Escape => KeyCode::Escape,
            Key::Tab => KeyCode::Tab,
            Key::Caps_Lock => KeyCode::CapsLock,
            Key::Shift_L | Key::Shift_R => KeyCode::Shift,
            Key::Control_L | Key::Control_R => KeyCode::Ctrl,
            Key::Alt_L | Key::Alt_R => KeyCode::Alt,
            Key::Super_L | Key::Super_R => KeyCode::Super,
            
            Key::space => KeyCode::Space,
            Key::Return => KeyCode::Enter,
            Key::BackSpace => KeyCode::Backspace,
            Key::Delete => KeyCode::Delete,
            
            Key::Left => KeyCode::Left,
            Key::Right => KeyCode::Right,
            Key::Up => KeyCode::Up,
            Key::Down => KeyCode::Down,
            
            Key::Home => KeyCode::Home,
            Key::End => KeyCode::End,
            Key::Page_Up => KeyCode::PageUp,
            Key::Page_Down => KeyCode::PageDown,
            
            Key::Insert => KeyCode::Insert,
            Key::Print => KeyCode::PrintScreen,
            Key::Scroll_Lock => KeyCode::ScrollLock,
            Key::Pause => KeyCode::Pause,
            
            _ => KeyCode::Unknown(0), // GTK4 Key doesn't provide raw value
        }
    }

    /// Convert modifier state to individual booleans
    pub fn parse_modifiers(state: gdk::ModifierType) -> (bool, bool, bool, bool) {
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        let alt = state.contains(gdk::ModifierType::ALT_MASK);
        let super_key = state.contains(gdk::ModifierType::SUPER_MASK);
        
        (shift, ctrl, alt, super_key)
    }

    /// Convert GTK button number to MouseButton
    pub fn gtk_button_to_mouse_button(button: u32) -> MouseButton {
        match button {
            1 => MouseButton::Left,
            2 => MouseButton::Middle,
            3 => MouseButton::Right,
            8 => MouseButton::X1, // Back button
            9 => MouseButton::X2, // Forward button
            _ => MouseButton::Left, // Default to left for unknown buttons
        }
    }
}

impl InputHandler for Gtk4Input {
    fn handle_keyboard(&mut self, _event: LegacyKeyboardEvent) -> Result<()> {
        // This is handled by the GTK4 event system in the application
        Ok(())
    }
    
    fn handle_mouse(&mut self, _event: MouseEvent) -> Result<()> {
        // This is handled by the GTK4 event system in the application
        Ok(())
    }
    
    fn grab_input(&mut self, grab: bool) -> Result<()> {
        if let Ok(mut grabbed) = self.grabbed.lock() {
            *grabbed = grab;
        }
        // Note: Actual pointer grabbing should be done at the widget level
        Ok(())
    }
    
    fn is_grabbed(&self) -> bool {
        self.grabbed.lock().map(|g| *g).unwrap_or(false)
    }
    
    fn set_relative_mouse(&mut self, relative: bool) -> Result<()> {
        if let Ok(mut rel) = self.relative_mode.lock() {
            *rel = relative;
        }
        Ok(())
    }
    
    fn warp_mouse(&mut self, _x: i32, _y: i32) -> Result<()> {
        // GTK4 doesn't provide direct mouse warping
        // This would need to be implemented at the window level if needed
        Ok(())
    }
}