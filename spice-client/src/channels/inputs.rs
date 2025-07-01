//! Inputs channel implementation for keyboard and mouse events

use crate::channels::{Channel, ChannelConnection, InputEvent, MouseButton, KeyCode};
use crate::error::Result;
use crate::protocol::*;
use tracing::{debug, info, warn};

/// Mouse operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    Server,
    Client,
}

/// Inputs channel for sending keyboard and mouse events
pub struct InputsChannel {
    pub(crate) connection: ChannelConnection,
    mouse_mode: MouseMode,
    modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl InputsChannel {
    pub async fn new(host: &str, port: u16, channel_id: u8) -> Result<Self> {
        Self::new_with_connection_id(host, port, channel_id, None).await
    }
    
    pub async fn new_with_connection_id(host: &str, port: u16, channel_id: u8, connection_id: Option<u32>) -> Result<Self> {
        let mut connection = ChannelConnection::new(host, port, ChannelType::Inputs, channel_id).await?;
        if let Some(conn_id) = connection_id {
            connection.set_connection_id(conn_id);
        }
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            mouse_mode: MouseMode::Server,
            modifiers: KeyModifiers::default(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(websocket_url: &str, channel_id: u8) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_id, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(websocket_url: &str, channel_id: u8, auth_token: Option<String>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Inputs, channel_id, auth_token).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            mouse_mode: MouseMode::Server,
            modifiers: KeyModifiers::default(),
        })
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth_and_session(websocket_url: &str, channel_id: u8, auth_token: Option<String>, password: Option<String>, connection_id: Option<u32>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Inputs, channel_id, auth_token).await?;
        if let Some(pwd) = password {
            connection.set_password(pwd);
        }
        if let Some(conn_id) = connection_id {
            connection.set_connection_id(conn_id);
        }
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            mouse_mode: MouseMode::Server,
            modifiers: KeyModifiers::default(),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Inputs channel {} initialized", self.connection.channel_id);
        Ok(())
    }

    pub fn get_mouse_mode(&self) -> MouseMode {
        self.mouse_mode
    }

    pub fn get_modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Sends an input event to the server
    pub async fn send_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::KeyDown(key) => {
                let scancode = key_to_scancode(key);
                self.update_modifiers(&key, true);
                self.send_key_down(scancode).await?
            },
            InputEvent::KeyUp(key) => {
                let scancode = key_to_scancode(key);
                self.update_modifiers(&key, false);
                self.send_key_up(scancode).await?
            },
            InputEvent::MouseMove { x, y } => self.send_mouse_motion(x, y).await?,
            InputEvent::MouseButton { button, pressed } => self.send_mouse_button(button, pressed).await?,
        }
        Ok(())
    }

    /// Sends a key down event with scancode
    pub async fn send_key_down(&mut self, scancode: u32) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&scancode.to_le_bytes());
        
        self.connection.send_message(SPICE_MSG_INPUTS_KEY_DOWN, &data).await?;
        debug!("Sent key down: scancode {}", scancode);
        Ok(())
    }

    /// Sends a key up event with scancode
    pub async fn send_key_up(&mut self, scancode: u32) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&scancode.to_le_bytes());
        
        self.connection.send_message(SPICE_MSG_INPUTS_KEY_UP, &data).await?;
        debug!("Sent key up: scancode {}", scancode);
        Ok(())
    }

    /// Sends a mouse motion event
    pub async fn send_mouse_motion(&mut self, x: i32, y: i32) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&x.to_le_bytes());
        data.extend_from_slice(&y.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes()); // button state
        
        self.connection.send_message(SPICE_MSG_INPUTS_MOUSE_MOTION, &data).await?;
        debug!("Sent mouse motion: ({}, {})", x, y);
        Ok(())
    }

    /// Sends a mouse button event
    pub async fn send_mouse_button(&mut self, button: MouseButton, pressed: bool) -> Result<()> {
        let button_mask = match button {
            MouseButton::Left => SPICE_MOUSE_BUTTON_LEFT,
            MouseButton::Middle => SPICE_MOUSE_BUTTON_MIDDLE,
            MouseButton::Right => SPICE_MOUSE_BUTTON_RIGHT,
            MouseButton::WheelUp => SPICE_MOUSE_BUTTON_WHEEL_UP,
            MouseButton::WheelDown => SPICE_MOUSE_BUTTON_WHEEL_DOWN,
        };
        
        let msg_type = if pressed {
            SPICE_MSG_INPUTS_MOUSE_PRESS
        } else {
            SPICE_MSG_INPUTS_MOUSE_RELEASE
        };
        
        let mut data = Vec::new();
        data.extend_from_slice(&button_mask.to_le_bytes());
        
        self.connection.send_message(msg_type, &data).await?;
        debug!("Sent mouse button: {:?} {}", button, if pressed { "pressed" } else { "released" });
        Ok(())
    }

    /// Updates modifier keys state
    fn update_modifiers(&mut self, key: &KeyCode, pressed: bool) {
        match key {
            KeyCode::Other(scancode) => {
                match *scancode {
                    0x2A | 0x36 => self.modifiers.shift = pressed, // Left/Right Shift
                    0x1D | 0x9D => self.modifiers.ctrl = pressed,  // Left/Right Ctrl
                    0x38 | 0xB8 => self.modifiers.alt = pressed,   // Left/Right Alt
                    0x5B | 0x5C => self.modifiers.meta = pressed,  // Left/Right Meta
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let (header, data) = self.connection.read_message().await?;
            self.handle_message(&header, &data).await?;
        }
    }

    async fn handle_init_message(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 2 {
            let modifiers = u16::from_le_bytes([data[0], data[1]]);
            info!("Inputs init - modifiers: 0x{:04X}", modifiers);
            
            // Update modifier state based on init message
            self.modifiers.shift = (modifiers & SPICE_KEYBOARD_MODIFIER_SHIFT) != 0;
            self.modifiers.ctrl = (modifiers & SPICE_KEYBOARD_MODIFIER_CTRL) != 0;
            self.modifiers.alt = (modifiers & SPICE_KEYBOARD_MODIFIER_ALT) != 0;
        }
        Ok(())
    }

    async fn handle_modifiers_message(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 2 {
            let modifiers = u16::from_le_bytes([data[0], data[1]]);
            debug!("Modifiers update: 0x{:04X}", modifiers);
            
            self.modifiers.shift = (modifiers & SPICE_KEYBOARD_MODIFIER_SHIFT) != 0;
            self.modifiers.ctrl = (modifiers & SPICE_KEYBOARD_MODIFIER_CTRL) != 0;
            self.modifiers.alt = (modifiers & SPICE_KEYBOARD_MODIFIER_ALT) != 0;
        }
        Ok(())
    }
}

impl Channel for InputsChannel {
    async fn handle_message(&mut self, header: &SpiceDataHeader, data: &[u8]) -> Result<()> {
        match header.msg_type {
            SPICE_MSG_INPUTS_INIT => {
                debug!("Received inputs init");
                self.handle_init_message(data).await?;
            }
            SPICE_MSG_INPUTS_KEY_MODIFIERS => {
                debug!("Received key modifiers");
                self.handle_modifiers_message(data).await?;
            }
            _ => {
                warn!("Unknown inputs message type: {}", header.msg_type);
            }
        }
        
        Ok(())
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Inputs
    }
}

// Inputs channel message types
pub const SPICE_MSG_INPUTS_INIT: u16 = 101;
pub const SPICE_MSG_INPUTS_KEY_MODIFIERS: u16 = 102;

// Client to server messages
pub const SPICE_MSG_INPUTS_KEY_DOWN: u16 = 103;
pub const SPICE_MSG_INPUTS_KEY_UP: u16 = 104;
pub const SPICE_MSG_INPUTS_MOUSE_MOTION: u16 = 105;
pub const SPICE_MSG_INPUTS_MOUSE_POSITION: u16 = 106;
pub const SPICE_MSG_INPUTS_MOUSE_PRESS: u16 = 107;
pub const SPICE_MSG_INPUTS_MOUSE_RELEASE: u16 = 108;

// Mouse button masks
pub const SPICE_MOUSE_BUTTON_LEFT: u32 = 1 << 0;
pub const SPICE_MOUSE_BUTTON_MIDDLE: u32 = 1 << 1;
pub const SPICE_MOUSE_BUTTON_RIGHT: u32 = 1 << 2;
pub const SPICE_MOUSE_BUTTON_WHEEL_UP: u32 = 1 << 3;   // Button 4
pub const SPICE_MOUSE_BUTTON_WHEEL_DOWN: u32 = 1 << 4; // Button 5

// Keyboard modifier masks
pub const SPICE_KEYBOARD_MODIFIER_SHIFT: u16 = 1 << 0;
pub const SPICE_KEYBOARD_MODIFIER_CTRL: u16 = 1 << 1;
pub const SPICE_KEYBOARD_MODIFIER_ALT: u16 = 1 << 2;

/// Converts a KeyCode to a PC scancode
fn key_to_scancode(key: KeyCode) -> u32 {
    match key {
        KeyCode::Escape => 0x01,
        KeyCode::Enter => 0x1C,
        KeyCode::Space => 0x39,
        KeyCode::Tab => 0x0F,
        KeyCode::Backspace => 0x0E,
        KeyCode::Function(n) => match n {
            1 => 0x3B,
            2 => 0x3C,
            3 => 0x3D,
            4 => 0x3E,
            5 => 0x3F,
            6 => 0x40,
            7 => 0x41,
            8 => 0x42,
            9 => 0x43,
            10 => 0x44,
            11 => 0x57,
            12 => 0x58,
            _ => 0x00,
        },
        KeyCode::ArrowUp => 0x48,
        KeyCode::ArrowDown => 0x50,
        KeyCode::ArrowLeft => 0x4B,
        KeyCode::ArrowRight => 0x4D,
        KeyCode::Char(c) => char_to_scancode(c),
        KeyCode::Other(scancode) => scancode,
    }
}

/// Converts a character to a PC scancode
fn char_to_scancode(c: char) -> u32 {
    match c.to_ascii_uppercase() {
        'A' => 0x1E,
        'B' => 0x30,
        'C' => 0x2E,
        'D' => 0x20,
        'E' => 0x12,
        'F' => 0x21,
        'G' => 0x22,
        'H' => 0x23,
        'I' => 0x17,
        'J' => 0x24,
        'K' => 0x25,
        'L' => 0x26,
        'M' => 0x32,
        'N' => 0x31,
        'O' => 0x18,
        'P' => 0x19,
        'Q' => 0x10,
        'R' => 0x13,
        'S' => 0x1F,
        'T' => 0x14,
        'U' => 0x16,
        'V' => 0x2F,
        'W' => 0x11,
        'X' => 0x2D,
        'Y' => 0x15,
        'Z' => 0x2C,
        '1' => 0x02,
        '2' => 0x03,
        '3' => 0x04,
        '4' => 0x05,
        '5' => 0x06,
        '6' => 0x07,
        '7' => 0x08,
        '8' => 0x09,
        '9' => 0x0A,
        '0' => 0x0B,
        '-' => 0x0C,
        '=' => 0x0D,
        '[' => 0x1A,
        ']' => 0x1B,
        ';' => 0x27,
        '\'' => 0x28,
        ',' => 0x33,
        '.' => 0x34,
        '/' => 0x35,
        '\\' => 0x2B,
        '`' => 0x29,
        _ => 0x00, // Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_scancode() {
        assert_eq!(key_to_scancode(KeyCode::Escape), 0x01);
        assert_eq!(key_to_scancode(KeyCode::Enter), 0x1C);
        assert_eq!(key_to_scancode(KeyCode::Space), 0x39);
        assert_eq!(key_to_scancode(KeyCode::Char('A')), 0x1E);
        assert_eq!(key_to_scancode(KeyCode::Char('a')), 0x1E);
        assert_eq!(key_to_scancode(KeyCode::Other(0x42)), 0x42);
    }

    #[test]
    fn test_modifiers() {
        let mut modifiers = KeyModifiers::default();
        assert!(!modifiers.shift);
        assert!(!modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
        
        modifiers.shift = true;
        modifiers.ctrl = true;
        assert!(modifiers.shift);
        assert!(modifiers.ctrl);
    }
}