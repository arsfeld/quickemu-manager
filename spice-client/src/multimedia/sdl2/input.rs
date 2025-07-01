use crate::multimedia::{
    input::{InputHandler, KeyCode, LegacyKeyboardEvent, MouseButton, MouseEvent},
    Result,
};
use sdl2::{
    event::Event,
    keyboard::Keycode,
    mouse::MouseButton as SdlMouseButton,
    Sdl,
};
use std::sync::Arc;

pub struct Sdl2Input {
    sdl_context: Arc<Sdl>,
    grabbed: bool,
    relative_mode: bool,
}

// Safety: SDL2 input handling is thread-safe when accessed properly
unsafe impl Send for Sdl2Input {}

impl Sdl2Input {
    pub fn new(sdl_context: Arc<Sdl>) -> Result<Self> {
        Ok(Self {
            sdl_context,
            grabbed: false,
            relative_mode: false,
        })
    }

    fn keycode_from_sdl(&self, keycode: Keycode) -> KeyCode {
        match keycode {
            Keycode::A => KeyCode::A,
            Keycode::B => KeyCode::B,
            Keycode::C => KeyCode::C,
            Keycode::D => KeyCode::D,
            Keycode::E => KeyCode::E,
            Keycode::F => KeyCode::F,
            Keycode::G => KeyCode::G,
            Keycode::H => KeyCode::H,
            Keycode::I => KeyCode::I,
            Keycode::J => KeyCode::J,
            Keycode::K => KeyCode::K,
            Keycode::L => KeyCode::L,
            Keycode::M => KeyCode::M,
            Keycode::N => KeyCode::N,
            Keycode::O => KeyCode::O,
            Keycode::P => KeyCode::P,
            Keycode::Q => KeyCode::Q,
            Keycode::R => KeyCode::R,
            Keycode::S => KeyCode::S,
            Keycode::T => KeyCode::T,
            Keycode::U => KeyCode::U,
            Keycode::V => KeyCode::V,
            Keycode::W => KeyCode::W,
            Keycode::X => KeyCode::X,
            Keycode::Y => KeyCode::Y,
            Keycode::Z => KeyCode::Z,
            Keycode::Num0 => KeyCode::Num0,
            Keycode::Num1 => KeyCode::Num1,
            Keycode::Num2 => KeyCode::Num2,
            Keycode::Num3 => KeyCode::Num3,
            Keycode::Num4 => KeyCode::Num4,
            Keycode::Num5 => KeyCode::Num5,
            Keycode::Num6 => KeyCode::Num6,
            Keycode::Num7 => KeyCode::Num7,
            Keycode::Num8 => KeyCode::Num8,
            Keycode::Num9 => KeyCode::Num9,
            Keycode::F1 => KeyCode::F1,
            Keycode::F2 => KeyCode::F2,
            Keycode::F3 => KeyCode::F3,
            Keycode::F4 => KeyCode::F4,
            Keycode::F5 => KeyCode::F5,
            Keycode::F6 => KeyCode::F6,
            Keycode::F7 => KeyCode::F7,
            Keycode::F8 => KeyCode::F8,
            Keycode::F9 => KeyCode::F9,
            Keycode::F10 => KeyCode::F10,
            Keycode::F11 => KeyCode::F11,
            Keycode::F12 => KeyCode::F12,
            Keycode::Escape => KeyCode::Escape,
            Keycode::Tab => KeyCode::Tab,
            Keycode::CapsLock => KeyCode::CapsLock,
            Keycode::LShift | Keycode::RShift => KeyCode::Shift,
            Keycode::LCtrl | Keycode::RCtrl => KeyCode::Ctrl,
            Keycode::LAlt | Keycode::RAlt => KeyCode::Alt,
            Keycode::LGui | Keycode::RGui => KeyCode::Super,
            Keycode::Space => KeyCode::Space,
            Keycode::Return => KeyCode::Enter,
            Keycode::Backspace => KeyCode::Backspace,
            Keycode::Delete => KeyCode::Delete,
            Keycode::Left => KeyCode::Left,
            Keycode::Right => KeyCode::Right,
            Keycode::Up => KeyCode::Up,
            Keycode::Down => KeyCode::Down,
            Keycode::Home => KeyCode::Home,
            Keycode::End => KeyCode::End,
            Keycode::PageUp => KeyCode::PageUp,
            Keycode::PageDown => KeyCode::PageDown,
            Keycode::Insert => KeyCode::Insert,
            Keycode::PrintScreen => KeyCode::PrintScreen,
            Keycode::ScrollLock => KeyCode::ScrollLock,
            Keycode::Pause => KeyCode::Pause,
            _ => KeyCode::Unknown(0), // SDL2 keycodes are not directly convertible to u32
        }
    }

    fn mouse_button_from_sdl(&self, button: SdlMouseButton) -> MouseButton {
        match button {
            SdlMouseButton::Left => MouseButton::Left,
            SdlMouseButton::Middle => MouseButton::Middle,
            SdlMouseButton::Right => MouseButton::Right,
            SdlMouseButton::X1 => MouseButton::X1,
            SdlMouseButton::X2 => MouseButton::X2,
            _ => MouseButton::Left, // Default fallback
        }
    }

    pub fn process_sdl_event(&mut self, event: Event) -> Option<Result<()>> {
        match event {
            Event::KeyDown { keycode: Some(keycode), keymod, .. } => {
                let key_event = LegacyKeyboardEvent {
                    key: self.keycode_from_sdl(keycode),
                    pressed: true,
                    shift: keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) || 
                           keymod.contains(sdl2::keyboard::Mod::RSHIFTMOD),
                    ctrl: keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) || 
                          keymod.contains(sdl2::keyboard::Mod::RCTRLMOD),
                    alt: keymod.contains(sdl2::keyboard::Mod::LALTMOD) || 
                         keymod.contains(sdl2::keyboard::Mod::RALTMOD),
                    super_key: keymod.contains(sdl2::keyboard::Mod::LGUIMOD) || 
                               keymod.contains(sdl2::keyboard::Mod::RGUIMOD),
                };
                Some(self.handle_keyboard(key_event))
            }
            Event::KeyUp { keycode: Some(keycode), keymod, .. } => {
                let key_event = LegacyKeyboardEvent {
                    key: self.keycode_from_sdl(keycode),
                    pressed: false,
                    shift: keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) || 
                           keymod.contains(sdl2::keyboard::Mod::RSHIFTMOD),
                    ctrl: keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) || 
                          keymod.contains(sdl2::keyboard::Mod::RCTRLMOD),
                    alt: keymod.contains(sdl2::keyboard::Mod::LALTMOD) || 
                         keymod.contains(sdl2::keyboard::Mod::RALTMOD),
                    super_key: keymod.contains(sdl2::keyboard::Mod::LGUIMOD) || 
                               keymod.contains(sdl2::keyboard::Mod::RGUIMOD),
                };
                Some(self.handle_keyboard(key_event))
            }
            Event::MouseMotion { x, y, xrel, yrel, .. } => {
                let mouse_event = MouseEvent::Motion {
                    x: x as u32,
                    y: y as u32,
                    relative_x: xrel,
                    relative_y: yrel,
                };
                Some(self.handle_mouse(mouse_event))
            }
            Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                let mouse_event = MouseEvent::Button {
                    button: self.mouse_button_from_sdl(mouse_btn),
                    pressed: true,
                    x: x as u32,
                    y: y as u32,
                };
                Some(self.handle_mouse(mouse_event))
            }
            Event::MouseButtonUp { mouse_btn, x, y, .. } => {
                let mouse_event = MouseEvent::Button {
                    button: self.mouse_button_from_sdl(mouse_btn),
                    pressed: false,
                    x: x as u32,
                    y: y as u32,
                };
                Some(self.handle_mouse(mouse_event))
            }
            Event::MouseWheel { x, y, .. } => {
                let mouse_event = MouseEvent::Wheel {
                    delta_x: x,
                    delta_y: y,
                };
                Some(self.handle_mouse(mouse_event))
            }
            _ => None,
        }
    }
}

impl InputHandler for Sdl2Input {
    fn handle_keyboard(&mut self, event: LegacyKeyboardEvent) -> Result<()> {
        // This would be connected to the SPICE protocol keyboard handling
        // For now, just return Ok
        Ok(())
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> Result<()> {
        // This would be connected to the SPICE protocol mouse handling
        // For now, just return Ok
        Ok(())
    }

    fn grab_input(&mut self, grab: bool) -> Result<()> {
        self.grabbed = grab;
        self.sdl_context.mouse().capture(grab);
        Ok(())
    }

    fn is_grabbed(&self) -> bool {
        self.grabbed
    }

    fn set_relative_mouse(&mut self, relative: bool) -> Result<()> {
        self.relative_mode = relative;
        self.sdl_context.mouse().set_relative_mouse_mode(relative);
        Ok(())
    }

    fn warp_mouse(&mut self, _x: i32, _y: i32) -> Result<()> {
        // Mouse warping requires a window reference, which we don't have here
        // This would need to be handled at a higher level
        Ok(())
    }
}