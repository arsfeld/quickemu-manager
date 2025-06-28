pub mod main;
pub mod display;

use crate::error::{Result, SpiceError};
use crate::protocol::*;

pub use main::MainChannel;
pub use display::{DisplayChannel, DisplaySurface};

/// Input event types
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// Key press event
    KeyDown(KeyCode),
    /// Key release event
    KeyUp(KeyCode),
    /// Mouse move event
    MouseMove { x: i32, y: i32 },
    /// Mouse button event
    MouseButton { button: MouseButton, pressed: bool },
}

/// Mouse button types
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Middle mouse button
    Middle,
    /// Right mouse button
    Right,
}

/// Key codes (simplified for example)
#[derive(Debug, Clone, Copy)]
pub enum KeyCode {
    /// Escape key
    Escape,
    /// Enter key
    Enter,
    /// Space key
    Space,
    /// Character key
    Char(char),
    /// Other key code
    Other(u32),
}
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::JsFuture,
    web_sys::*,
    js_sys::{ArrayBuffer, Uint8Array},
    std::sync::{Arc, Mutex},
};

pub trait Channel {
    async fn handle_message(&mut self, header: &SpiceDataHeader, data: &[u8]) -> Result<()>;
    fn channel_type(&self) -> ChannelType;
}

pub struct ChannelConnection {
    #[cfg(not(target_arch = "wasm32"))]
    stream: TcpStream,
    #[cfg(target_arch = "wasm32")]
    websocket: Option<Arc<Mutex<WebSocket>>>,
    #[cfg(target_arch = "wasm32")]
    byte_buffer: Arc<Mutex<Vec<u8>>>,
    channel_type: ChannelType,
    pub channel_id: u8,
}

impl ChannelConnection {
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new(
        host: &str,
        port: u16,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        let stream = TcpStream::connect((host, port)).await?;
        
        Ok(Self {
            stream,
            channel_type,
            channel_id,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        host: &str,
        port: u16,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        let websocket_url = format!("ws://{}:{}", host, port);
        Self::new_websocket(&websocket_url, channel_type, channel_id).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(
        websocket_url: &str,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_type, channel_id, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(
        websocket_url: &str,
        channel_type: ChannelType,
        channel_id: u8,
        auth_token: Option<String>,
    ) -> Result<Self> {
        let window = web_sys::window().ok_or_else(|| SpiceError::Protocol("No window object".to_string()))?;
        let websocket = WebSocket::new(websocket_url)
            .map_err(|e| SpiceError::Protocol(format!("Failed to create WebSocket: {:?}", e)))?;
        
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);
        
        let byte_buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&byte_buffer);
        let auth_response = Arc::new(Mutex::new(String::new()));
        let auth_response_clone = Arc::clone(&auth_response);
        
        // Set up message handler - handle both text (auth) and binary (SPICE) messages
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Try text message first (for authentication response)
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let text_str = text.as_string().unwrap_or_default();
                if let Ok(mut auth_buf) = auth_response_clone.lock() {
                    *auth_buf = text_str;
                }
            }
            // Try binary message (for SPICE protocol data)
            else if let Ok(arraybuffer) = e.data().dyn_into::<ArrayBuffer>() {
                let array = Uint8Array::new(&arraybuffer);
                let mut bytes = vec![0u8; array.length() as usize];
                array.copy_to(&mut bytes);
                
                if let Ok(mut buffer) = buffer_clone.lock() {
                    buffer.extend_from_slice(&bytes);
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
        
        // Wait for connection to open
        let ready_state_check = || {
            websocket.ready_state() == WebSocket::OPEN
        };
        
        // Simple polling for connection open
        let mut attempts = 0;
        while !ready_state_check() && attempts < 100 {
            gloo_timers::future::TimeoutFuture::new(50).await;
            attempts += 1;
        }
        
        if !ready_state_check() {
            return Err(SpiceError::Protocol("WebSocket connection timeout".to_string()));
        }
        
        // Send authentication token if provided
        if let Some(token) = auth_token {
            info!("Sending auth token: {}", token);
            let ws_clone = websocket.clone();
            ws_clone.send_with_str(&token)
                .map_err(|e| SpiceError::Protocol(format!("Failed to send auth token: {:?}", e)))?;
                
            // Wait for authentication response
            let mut attempts = 0;
            let mut auth_success = false;
            
            while attempts < 200 && !auth_success { // Increased timeout
                if let Ok(auth_buf) = auth_response.lock() {
                    if !auth_buf.is_empty() {
                        info!("Received auth response: '{}'", auth_buf);
                        if auth_buf.contains("OK") {
                            info!("Authentication successful");
                            auth_success = true;
                        } else if auth_buf.contains("Authentication failed") {
                            return Err(SpiceError::Protocol("WebSocket authentication failed".to_string()));
                        } else {
                            info!("Unexpected auth response: '{}'", auth_buf);
                        }
                    }
                }
                if !auth_success {
                    gloo_timers::future::TimeoutFuture::new(50).await;
                    attempts += 1;
                }
            }
            
            if !auth_success {
                return Err(SpiceError::Protocol("WebSocket authentication timeout".to_string()));
            }
            
            // Clear any residual data in the byte buffer after authentication
            if let Ok(mut buffer) = byte_buffer.lock() {
                buffer.clear();
                info!("Cleared byte buffer after authentication");
            }
        } else {
            info!("No auth token provided, skipping authentication");
        }
        
        Ok(Self {
            websocket: Some(Arc::new(Mutex::new(websocket))),
            byte_buffer,
            channel_type,
            channel_id,
        })
    }

    pub async fn handshake(&mut self) -> Result<()> {
        // Send link header
        let link_header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: std::mem::size_of::<SpiceLinkMess>() as u32,
        };

        let header_bytes = bincode::serialize(&link_header)
            .map_err(|e| SpiceError::Protocol(format!("Failed to serialize link header: {}", e)))?;
        
        info!("Sending SPICE link header: {:?}", header_bytes);
        self.send_raw(&header_bytes).await?;

        // Send link message
        let link_mess = SpiceLinkMess {
            connection_id: 0,
            channel_type: self.channel_type as u8,
            channel_id: self.channel_id,
            num_common_caps: 0,
            num_channel_caps: 0,
            caps_offset: std::mem::size_of::<SpiceLinkMess>() as u32,
        };

        let mess_bytes = bincode::serialize(&link_mess)
            .map_err(|e| SpiceError::Protocol(format!("Failed to serialize link message: {}", e)))?;
        
        self.send_raw(&mess_bytes).await?;

        // Read reply
        let reply_bytes = self.read_raw(std::mem::size_of::<SpiceLinkReply>()).await?;

        // Debug: log the raw bytes we received
        info!("Received reply bytes: {:?}", reply_bytes);
        if reply_bytes.len() >= 4 {
            let magic_bytes = &reply_bytes[0..4];
            let magic = u32::from_le_bytes([magic_bytes[0], magic_bytes[1], magic_bytes[2], magic_bytes[3]]);
            info!("Magic in reply: 0x{:08x}, expected: 0x{:08x}", magic, SPICE_MAGIC);
        }

        let reply: SpiceLinkReply = bincode::deserialize(&reply_bytes)
            .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize link reply: {}", e)))?;

        if reply.magic != SPICE_MAGIC && reply.magic != crate::protocol::SPICE_MAGIC_REDQ {
            return Err(SpiceError::Protocol(format!(
                "Invalid magic in reply: got 0x{:08x}, expected 0x{:08x} or 0x{:08x}", 
                reply.magic, SPICE_MAGIC, crate::protocol::SPICE_MAGIC_REDQ
            )));
        }

        // Read the link message data if size > 0
        if reply.size > 0 {
            info!("Reading {} bytes of link message data", reply.size);
            let link_data = self.read_raw(reply.size as usize).await?;
            info!("Link message data: {:?}", link_data);
        }

        Ok(())
    }

    async fn send_raw(&mut self, data: &[u8]) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stream.write_all(data).await?;
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ref ws) = self.websocket {
                if let Ok(websocket) = ws.lock() {
                    websocket.send_with_u8_array(data)
                        .map_err(|e| SpiceError::Protocol(format!("Failed to send WebSocket data: {:?}", e)))?;
                }
            }
        }
        
        Ok(())
    }

    async fn read_raw(&mut self, len: usize) -> Result<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut data = vec![0u8; len];
            self.stream.read_exact(&mut data).await?;
            Ok(data)
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Wait for enough data in byte buffer
            let mut attempts = 0;
            let mut last_buffer_size = 0;
            while attempts < 2000 { // Increased timeout for SPICE handshake
                if let Ok(mut buffer) = self.byte_buffer.lock() {
                    if buffer.len() >= len {
                        let data = buffer.drain(..len).collect();
                        info!("Read {} bytes from WebSocket: {:?}", len, data);
                        return Ok(data);
                    } else if !buffer.is_empty() {
                        if buffer.len() != last_buffer_size {
                            info!("Buffer has {} bytes, need {} (attempt {})", buffer.len(), len, attempts);
                            last_buffer_size = buffer.len();
                        }
                    }
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
                attempts += 1;
                
                // Log progress every 100 attempts
                if attempts % 100 == 0 {
                    info!("Still waiting for {} bytes, attempt {}/2000", len, attempts);
                }
            }
            info!("Timeout after {} attempts waiting for {} bytes", attempts, len);
            Err(SpiceError::Protocol("Timeout waiting for WebSocket data".to_string()))
        }
    }

    pub async fn read_message(&mut self) -> Result<(SpiceDataHeader, Vec<u8>)> {
        let header_bytes = self.read_raw(std::mem::size_of::<SpiceDataHeader>()).await?;

        let header: SpiceDataHeader = bincode::deserialize(&header_bytes)
            .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize data header: {}", e)))?;

        let data = self.read_raw(header.msg_size as usize).await?;

        Ok((header, data))
    }

    pub async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        let header = SpiceDataHeader {
            serial: 0, // Should be managed properly in a real implementation
            msg_type,
            msg_size: data.len() as u32,
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&header)
            .map_err(|e| SpiceError::Protocol(format!("Failed to serialize data header: {}", e)))?;

        info!("Sending message: type={}, size={}, header_bytes={:?}", msg_type, data.len(), header_bytes);
        self.send_raw(&header_bytes).await?;
        if !data.is_empty() {
            info!("Sending message data: {:?}", data);
            self.send_raw(data).await?;
        }

        Ok(())
    }
}