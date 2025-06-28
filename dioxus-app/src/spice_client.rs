// SPICE client bridge for WebAssembly
// This module provides a bridge between the spice-client library and the web environment

use tokio::sync::mpsc;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    web_sys::{WebSocket, MessageEvent, BinaryType, CloseEvent, ErrorEvent},
    wasm_bindgen::JsCast,
    js_sys::Uint8Array,
};

#[derive(Debug, Clone)]
pub enum SpiceMessage {
    Connect(String, Option<String>), // URL, optional auth token
    Disconnect,
    SendData(Vec<u8>),
    KeyDown(u32), // keycode
    KeyUp(u32),   // keycode
    MouseMove(i32, i32), // x, y
    MouseButton(u8, bool), // button, pressed
}

#[derive(Debug, Clone)]
pub enum SpiceEvent {
    Connected,
    Disconnected,
    Error(String),
    Data(Vec<u8>),
    DisplayUpdate {
        surface_id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
}

pub struct SpiceClient {
    #[cfg(target_arch = "wasm32")]
    websocket: Option<WebSocket>,
    event_tx: mpsc::UnboundedSender<SpiceEvent>,
    message_rx: mpsc::UnboundedReceiver<SpiceMessage>,
}

impl SpiceClient {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SpiceEvent>, mpsc::UnboundedSender<SpiceMessage>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        let client = Self {
            #[cfg(target_arch = "wasm32")]
            websocket: None,
            event_tx,
            message_rx,
        };
        
        (client, event_rx, message_tx)
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn run(mut self) {
        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                SpiceMessage::Connect(url, auth_token) => {
                    if let Err(e) = self.connect(&url, auth_token).await {
                        let _ = self.event_tx.send(SpiceEvent::Error(format!("Connection failed: {:?}", e)));
                    }
                }
                SpiceMessage::Disconnect => {
                    self.disconnect();
                }
                SpiceMessage::SendData(data) => {
                    if let Some(ws) = &self.websocket {
                        let array = Uint8Array::from(&data[..]);
                        if let Err(e) = ws.send_with_array_buffer(&array.buffer()) {
                            log::error!("Failed to send data: {:?}", e);
                        }
                    }
                }
                SpiceMessage::KeyDown(_keycode) => {
                    // TODO: Implement keyboard input handling
                    log::debug!("Key down event received");
                }
                SpiceMessage::KeyUp(_keycode) => {
                    // TODO: Implement keyboard input handling
                    log::debug!("Key up event received");
                }
                SpiceMessage::MouseMove(_x, _y) => {
                    // TODO: Implement mouse input handling
                    log::debug!("Mouse move event received");
                }
                SpiceMessage::MouseButton(_button, _pressed) => {
                    // TODO: Implement mouse input handling
                    log::debug!("Mouse button event received");
                }
            }
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run(mut self) {
        // Desktop version not implemented for SPICE viewer
        let _ = self.event_tx.send(SpiceEvent::Error("SPICE viewer is only available in web mode".to_string()));
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn connect(&mut self, url: &str, auth_token: Option<String>) -> Result<(), JsValue> {
        log::info!("SpiceClient: Connecting to {}", url);
        
        // Create WebSocket
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        
        let event_tx = self.event_tx.clone();
        let event_tx_clone = event_tx.clone();
        let event_tx_clone2 = event_tx.clone();
        let event_tx_clone3 = event_tx.clone();
        let auth_token_clone = auth_token.clone();
        
        // Set up event handlers
        let onopen = Closure::wrap(Box::new(move |_| {
            log::info!("SpiceClient: WebSocket opened");
            
            // Send auth token if provided
            if let Some(token) = &auth_token_clone {
                // TODO: Send auth token according to SPICE protocol
                log::debug!("SpiceClient: Would send auth token");
            }
            
            let _ = event_tx.send(SpiceEvent::Connected);
        }) as Box<dyn FnMut(JsValue)>);
        
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(data) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = Uint8Array::new(&data);
                let bytes = array.to_vec();
                log::debug!("SpiceClient: Received binary message, {} bytes", bytes.len());
                
                // TODO: Parse SPICE protocol messages
                // For now, just forward raw data
                let _ = event_tx_clone.send(SpiceEvent::Data(bytes));
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        
        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("SpiceClient: WebSocket error: {}", e.message());
            let _ = event_tx_clone2.send(SpiceEvent::Error(format!("WebSocket error: {}", e.message())));
        }) as Box<dyn FnMut(ErrorEvent)>);
        
        let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
            log::info!("SpiceClient: WebSocket closed with code {}", e.code());
            let _ = event_tx_clone3.send(SpiceEvent::Disconnected);
        }) as Box<dyn FnMut(CloseEvent)>);
        
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        
        // Keep closures alive
        onopen.forget();
        onmessage.forget();
        onerror.forget();
        onclose.forget();
        
        self.websocket = Some(ws);
        
        Ok(())
    }
    
    #[cfg(target_arch = "wasm32")]
    fn disconnect(&mut self) {
        if let Some(ws) = self.websocket.take() {
            let _ = ws.close();
        }
    }
}