pub mod main;
pub mod display;
pub mod cursor;
pub mod inputs;

#[cfg(target_arch = "wasm32")]
pub mod display_wasm;

#[cfg(test)]
mod tests;

use crate::error::{Result, SpiceError};
use crate::protocol::*;
use rsa::{Oaep, RsaPublicKey};
use rsa::pkcs8::DecodePublicKey;
use rand::rngs::OsRng;
use sha1::Sha1;

pub use main::MainChannel;
pub use display::{DisplayChannel, DisplaySurface};
pub use cursor::{CursorChannel, CursorShape};
pub use inputs::{InputsChannel, MouseMode, KeyModifiers};

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
use tracing::{info, warn};

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
    password: Option<String>,
}

/// Encrypt a password using RSA-OAEP with SHA-1
fn encrypt_password(password: &str, pub_key_der: &[u8]) -> Result<Vec<u8>> {
    // The SPICE server sends the public key in SubjectPublicKeyInfo DER format
    match RsaPublicKey::from_public_key_der(pub_key_der) {
        Ok(public_key) => {
            // SPICE uses RSA-OAEP with SHA-1
            let padding = Oaep::new::<Sha1>();
            match public_key.encrypt(&mut OsRng, padding, password.as_bytes()) {
                Ok(encrypted) => Ok(encrypted),
                Err(e) => Err(SpiceError::Protocol(format!("Failed to encrypt password: {}", e)))
            }
        }
        Err(e) => {
            warn!("Failed to parse RSA public key: {}, trying raw modulus/exponent", e);
            // The public key might be in a different format, let's try to parse it manually
            // SPICE sends: error(4) + pubkey(162) + caps...
            // The pubkey is in SubjectPublicKeyInfo format starting at offset 4
            Err(SpiceError::Protocol(format!("Failed to parse RSA public key: {}", e)))
        }
    }
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
            password: None,
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
            password: None,
        })
    }
    
    pub fn set_password(&mut self, password: String) {
        self.password = Some(password);
    }

    pub async fn handshake(&mut self) -> Result<()> {
        // First serialize the link message to get its actual size
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
        
        // Send link header with the actual serialized size
        let link_header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: mess_bytes.len() as u32,  // Use actual serialized size, not struct size
        };

        let header_bytes = bincode::serialize(&link_header)
            .map_err(|e| SpiceError::Protocol(format!("Failed to serialize link header: {}", e)))?;
        
        info!("Sending SPICE link header: {:?}", header_bytes);
        self.send_raw(&header_bytes).await?;
        
        self.send_raw(&mess_bytes).await?;

        // Read reply
        let reply_bytes = self.read_raw(std::mem::size_of::<SpiceLinkReply>()).await?;

        // Debug: log the raw bytes we received
        info!("Received reply bytes: {:?}", reply_bytes);
        if reply_bytes.len() >= 4 {
            let magic_bytes = &reply_bytes[0..4];
            let magic = u32::from_le_bytes([magic_bytes[0], magic_bytes[1], magic_bytes[2], magic_bytes[3]]);
            info!("Magic in reply: 0x{:08x}, expected: 0x{:08x} or 0x{:08x}", magic, SPICE_MAGIC, crate::protocol::SPICE_MAGIC_LEGACY);
        }

        let reply: SpiceLinkReply = bincode::deserialize(&reply_bytes)
            .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize link reply: {}", e)))?;

        if reply.magic != SPICE_MAGIC && reply.magic != crate::protocol::SPICE_MAGIC_LEGACY {
            return Err(SpiceError::Protocol(format!(
                "Invalid magic in reply: got 0x{:08x}, expected 0x{:08x} or 0x{:08x}", 
                reply.magic, SPICE_MAGIC, crate::protocol::SPICE_MAGIC_LEGACY
            )));
        }

        // Read the link message data if size > 0
        if reply.size > 0 {
            info!("Reading {} bytes of link message data", reply.size);
            let link_data = self.read_raw(reply.size as usize).await?;
            info!("Link message data: {:?}", link_data);
            
            // Parse the link reply data to determine authentication requirements
            if link_data.len() >= 4 {
                let error_code = u32::from_le_bytes([link_data[0], link_data[1], link_data[2], link_data[3]]);
                
                if error_code == 0 && link_data.len() >= 166 {
                    // Server sent public key at offset 4, length 162
                    let pub_key_der = &link_data[4..166];
                    info!("Server provided RSA public key (162 bytes)");
                    
                    // Determine what to encrypt based on whether we have a password
                    let password_to_encrypt = if let Some(ref password) = self.password {
                        info!("Password provided, encrypting it");
                        password.as_str()
                    } else {
                        info!("No password provided, encrypting empty string");
                        ""
                    };
                    
                    // Encrypt the password (or empty string)
                    match encrypt_password(password_to_encrypt, pub_key_der) {
                        Ok(encrypted_password) => {
                            info!("Successfully encrypted password, sending {} bytes", encrypted_password.len());
                            self.send_raw(&encrypted_password).await?;
                        }
                        Err(e) => {
                            warn!("Failed to encrypt password: {}, sending zeros as fallback", e);
                            let zeros = vec![0u8; 128];
                            self.send_raw(&zeros).await?;
                        }
                    }
                    
                    // Read link result after authentication
                    info!("Reading link result after authentication");
                    let link_result = self.read_raw(4).await?;
                    let auth_error = u32::from_le_bytes([link_result[0], link_result[1], link_result[2], link_result[3]]);
                    
                    if auth_error != 0 {
                        return Err(SpiceError::Protocol(format!(
                            "Authentication failed with error code: {} ({})", 
                            auth_error,
                            match auth_error {
                                7 => "PERMISSION_DENIED",
                                5 => "NEED_SECURED",
                                6 => "NEED_UNSECURED",
                                _ => "UNKNOWN_ERROR"
                            }
                        )));
                    }
                    info!("Authentication successful");
                } else if error_code != 0 {
                    return Err(SpiceError::Protocol(format!(
                        "Link stage failed with error code: {}", error_code
                    )));
                }
            }
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

        // Pad header to 24 bytes to match what the server expects
        let mut padded_header = vec![0u8; 24];
        padded_header[..header_bytes.len()].copy_from_slice(&header_bytes);

        info!("Sending message: type={}, size={}, header_bytes={:?}", msg_type, data.len(), padded_header);
        self.send_raw(&padded_header).await?;
        if !data.is_empty() {
            info!("Sending message data: {:?}", data);
            self.send_raw(data).await?;
        }

        Ok(())
    }
}