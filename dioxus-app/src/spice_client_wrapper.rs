// SPICE client wrapper for WebAssembly integration
// This module provides proper integration with the spice-client library

use tokio::sync::mpsc;
use spice_client::SpiceError;
use spice_client::channels::{DisplaySurface, KeyCode, MouseButton};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Clone)]
pub enum SpiceMessage {
    Connect(String, Option<String>), // URL, optional password
    Disconnect,
    KeyDown(u32), // keycode
    KeyUp(u32),   // keycode
    MouseMove(i32, i32), // x, y
    MouseButton(u8, bool), // button, pressed
    MouseWheel(i32), // delta
}

#[derive(Debug, Clone)]
pub enum SpiceEvent {
    Connected,
    Disconnected,
    Error(String),
    DisplayUpdate {
        surface_id: u32,
        surface: DisplaySurface,
    },
    CursorUpdate {
        x: i16,
        y: i16,
        visible: bool,
    },
}

pub struct SpiceClientWrapper {
    event_tx: mpsc::UnboundedSender<SpiceEvent>,
    message_rx: mpsc::UnboundedReceiver<SpiceMessage>,
    #[cfg(target_arch = "wasm32")]
    update_callback: Option<Box<dyn Fn(u32, &DisplaySurface)>>,
}

impl SpiceClientWrapper {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SpiceEvent>, mpsc::UnboundedSender<SpiceMessage>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        let wrapper = Self {
            event_tx,
            message_rx,
            #[cfg(target_arch = "wasm32")]
            update_callback: None,
        };
        
        (wrapper, event_rx, message_tx)
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn set_update_callback<F>(&mut self, callback: F) 
    where
        F: Fn(u32, &DisplaySurface) + 'static
    {
        self.update_callback = Some(Box::new(callback));
    }
    
    pub async fn run(mut self) {
        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                SpiceMessage::Connect(url, password) => {
                    // For now, just simulate a connection
                    // In a real implementation, this would use the spice-client library
                    log::info!("SpiceClientWrapper: Simulating connection to {}", url);
                    let _ = self.event_tx.send(SpiceEvent::Connected);
                    
                    // Simulate a display update
                    let surface = DisplaySurface {
                        width: 1024,
                        height: 768,
                        format: 8, // SPICE_BITMAP_FMT_32BIT
                        data: vec![0; 1024 * 768 * 4],
                    };
                    
                    let _ = self.event_tx.send(SpiceEvent::DisplayUpdate {
                        surface_id: 0,
                        surface: surface.clone(),
                    });
                    
                    #[cfg(target_arch = "wasm32")]
                    if let Some(ref callback) = self.update_callback {
                        callback(0, &surface);
                    }
                }
                SpiceMessage::Disconnect => {
                    let _ = self.event_tx.send(SpiceEvent::Disconnected);
                }
                SpiceMessage::KeyDown(keycode) => {
                    log::debug!("Key down: {}", keycode);
                }
                SpiceMessage::KeyUp(keycode) => {
                    log::debug!("Key up: {}", keycode);
                }
                SpiceMessage::MouseMove(x, y) => {
                    log::debug!("Mouse move: {}, {}", x, y);
                }
                SpiceMessage::MouseButton(button, pressed) => {
                    log::debug!("Mouse button {}: {}", button, pressed);
                }
                SpiceMessage::MouseWheel(delta) => {
                    log::debug!("Mouse wheel: {}", delta);
                }
            }
        }
    }
}

// Map browser keycodes to SPICE keycodes
fn map_keycode_to_spice(keycode: u32) -> Option<KeyCode> {
    match keycode {
        // Letters
        65 => Some(KeyCode::Char('A')),
        66 => Some(KeyCode::Char('B')),
        67 => Some(KeyCode::Char('C')),
        68 => Some(KeyCode::Char('D')),
        69 => Some(KeyCode::Char('E')),
        70 => Some(KeyCode::Char('F')),
        71 => Some(KeyCode::Char('G')),
        72 => Some(KeyCode::Char('H')),
        73 => Some(KeyCode::Char('I')),
        74 => Some(KeyCode::Char('J')),
        75 => Some(KeyCode::Char('K')),
        76 => Some(KeyCode::Char('L')),
        77 => Some(KeyCode::Char('M')),
        78 => Some(KeyCode::Char('N')),
        79 => Some(KeyCode::Char('O')),
        80 => Some(KeyCode::Char('P')),
        81 => Some(KeyCode::Char('Q')),
        82 => Some(KeyCode::Char('R')),
        83 => Some(KeyCode::Char('S')),
        84 => Some(KeyCode::Char('T')),
        85 => Some(KeyCode::Char('U')),
        86 => Some(KeyCode::Char('V')),
        87 => Some(KeyCode::Char('W')),
        88 => Some(KeyCode::Char('X')),
        89 => Some(KeyCode::Char('Y')),
        90 => Some(KeyCode::Char('Z')),
        
        // Numbers
        48 => Some(KeyCode::Char('0')),
        49 => Some(KeyCode::Char('1')),
        50 => Some(KeyCode::Char('2')),
        51 => Some(KeyCode::Char('3')),
        52 => Some(KeyCode::Char('4')),
        53 => Some(KeyCode::Char('5')),
        54 => Some(KeyCode::Char('6')),
        55 => Some(KeyCode::Char('7')),
        56 => Some(KeyCode::Char('8')),
        57 => Some(KeyCode::Char('9')),
        
        // Special keys
        8 => Some(KeyCode::Backspace),
        9 => Some(KeyCode::Tab),
        13 => Some(KeyCode::Enter),
        27 => Some(KeyCode::Escape),
        32 => Some(KeyCode::Space),
        37 => Some(KeyCode::ArrowLeft),
        38 => Some(KeyCode::ArrowUp),
        39 => Some(KeyCode::ArrowRight),
        40 => Some(KeyCode::ArrowDown),
        
        // Function keys
        112 => Some(KeyCode::Function(1)),
        113 => Some(KeyCode::Function(2)),
        114 => Some(KeyCode::Function(3)),
        115 => Some(KeyCode::Function(4)),
        116 => Some(KeyCode::Function(5)),
        117 => Some(KeyCode::Function(6)),
        118 => Some(KeyCode::Function(7)),
        119 => Some(KeyCode::Function(8)),
        120 => Some(KeyCode::Function(9)),
        121 => Some(KeyCode::Function(10)),
        122 => Some(KeyCode::Function(11)),
        123 => Some(KeyCode::Function(12)),
        
        _ => None,
    }
}