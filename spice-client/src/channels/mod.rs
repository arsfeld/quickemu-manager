pub mod connection;
pub mod cursor;
pub mod display;
pub mod inputs;
pub mod main;

#[cfg(target_arch = "wasm32")]
pub mod display_wasm;

#[cfg(test)]
mod tests;

use crate::error::{Result, SpiceError};
use crate::protocol::*;
use rand::rngs::OsRng;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Oaep, RsaPublicKey};
use sha1::Sha1;

#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};
#[cfg(target_arch = "wasm32")]
use web_sys::WebSocket;

use tracing::{debug, info, warn};

pub use cursor::{CursorChannel, CursorShape};
pub use display::{DisplayChannel, DisplaySurface};
pub use inputs::{InputsChannel, KeyModifiers, MouseMode};
pub use main::MainChannel;

/// Input event types for keyboard and mouse interactions.
///
/// This enum represents all possible input events that can be sent to
/// the SPICE server through the inputs channel.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// A key was pressed down.
    KeyDown(KeyCode),
    /// A key was released.
    KeyUp(KeyCode),
    /// The mouse pointer moved to a new position.
    ///
    /// Coordinates are absolute positions within the display surface.
    MouseMove {
        /// X coordinate in pixels from the left edge
        x: i32,
        /// Y coordinate in pixels from the top edge
        y: i32,
    },
    /// A mouse button state changed.
    MouseButton {
        /// Which mouse button was affected
        button: MouseButton,
        /// Whether the button is now pressed (true) or released (false)
        pressed: bool,
    },
}

/// Mouse button identifiers.
///
/// Represents the standard mouse buttons that can be used with SPICE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left mouse button (primary button).
    Left,
    /// Middle mouse button (often the scroll wheel button).
    Middle,
    /// Right mouse button (secondary button).
    Right,
    /// Mouse wheel up (scroll up).
    WheelUp,
    /// Mouse wheel down (scroll down).
    WheelDown,
}

/// Keyboard key codes.
///
/// This is a simplified set of key codes for common keys. The SPICE protocol
/// uses PC/AT keyboard scan codes internally, but this enum provides a more
/// convenient abstraction for applications.
///
/// # Example
///
/// ```
/// use spice_client::KeyCode;
///
/// // Common keys have their own variants
/// let enter = KeyCode::Enter;
/// let escape = KeyCode::Escape;
///
/// // Letters and digits use the Char variant
/// let letter_a = KeyCode::Char('A');
/// let digit_5 = KeyCode::Char('5');
///
/// // Special keys use scan codes with the Other variant
/// let f1_key = KeyCode::Other(0x3B); // F1 scan code
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// Escape key.
    Escape,
    /// Enter/Return key.
    Enter,
    /// Space bar.
    Space,
    /// Tab key.
    Tab,
    /// Backspace key.
    Backspace,
    /// A character key (letters, digits, symbols).
    ///
    /// The character should be uppercase for letters.
    Char(char),
    /// Function keys (F1-F12).
    ///
    /// Use values 1-12 for F1-F12.
    Function(u8),
    /// Arrow keys.
    ArrowUp,
    /// Arrow down key.
    ArrowDown,
    /// Arrow left key.
    ArrowLeft,
    /// Arrow right key.
    ArrowRight,
    /// Other key specified by PC/AT scan code.
    ///
    /// This allows sending any key by its raw scan code value.
    Other(u32),
}

#[cfg(target_arch = "wasm32")]
use {
    js_sys::{ArrayBuffer, Uint8Array},
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::JsFuture,
    web_sys::*,
};

#[allow(async_fn_in_trait)]
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
    connection_id: Option<u32>,
    next_serial: u64,
    handshake_complete: bool,
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
                Err(e) => Err(SpiceError::Protocol(format!(
                    "Failed to encrypt password: {}",
                    e
                ))),
            }
        }
        Err(e) => {
            warn!(
                "Failed to parse RSA public key: {}, trying raw modulus/exponent",
                e
            );
            // The public key might be in a different format, let's try to parse it manually
            // SPICE sends: error(4) + pubkey(162) + caps...
            // The pubkey is in SubjectPublicKeyInfo format starting at offset 4
            Err(SpiceError::Protocol(format!(
                "Failed to parse RSA public key: {}",
                e
            )))
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
            connection_id: None,
            next_serial: 1,
            handshake_complete: false,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        host: &str,
        port: u16,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        let websocket_url = format!("ws://{host}:{port}");
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
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object".to_string()))?;
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
        let ready_state_check = || websocket.ready_state() == WebSocket::OPEN;

        // Simple polling for connection open
        let mut attempts = 0;
        while !ready_state_check() && attempts < 100 {
            gloo_timers::future::TimeoutFuture::new(50).await;
            attempts += 1;
        }

        if !ready_state_check() {
            return Err(SpiceError::Protocol(
                "WebSocket connection timeout".to_string(),
            ));
        }

        // Send authentication token if provided
        if let Some(token) = auth_token {
            info!("Sending auth token: {}", token);
            let ws_clone = websocket.clone();
            ws_clone
                .send_with_str(&token)
                .map_err(|e| SpiceError::Protocol(format!("Failed to send auth token: {:?}", e)))?;

            // Wait for authentication response
            let mut attempts = 0;
            let mut auth_success = false;

            while attempts < 200 && !auth_success {
                // Increased timeout
                if let Ok(auth_buf) = auth_response.lock() {
                    if !auth_buf.is_empty() {
                        info!("Received auth response: '{}'", auth_buf);
                        if auth_buf.contains("OK") {
                            info!("Authentication successful");
                            auth_success = true;
                        } else if auth_buf.contains("Authentication failed") {
                            return Err(SpiceError::Protocol(
                                "WebSocket authentication failed".to_string(),
                            ));
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
                return Err(SpiceError::Protocol(
                    "WebSocket authentication timeout".to_string(),
                ));
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
            connection_id: None,
            next_serial: 1,
            handshake_complete: false,
        })
    }

    pub fn set_password(&mut self, password: String) {
        self.password = Some(password);
    }

    pub fn set_connection_id(&mut self, connection_id: u32) {
        self.connection_id = Some(connection_id);
    }

    /// Convert a list of capability bits into a capability bitmap array
    fn encode_capabilities(caps: &[u32]) -> Vec<u32> {
        if caps.is_empty() {
            return vec![];
        }

        // Find the highest capability bit to determine array size
        let max_cap = caps.iter().max().unwrap_or(&0);
        let num_words = (max_cap / 32) + 1;
        let mut bitmap = vec![0u32; num_words as usize];

        // Set each capability bit
        for &cap in caps {
            let word_index = (cap / 32) as usize;
            let bit_index = cap % 32;
            bitmap[word_index] |= 1u32 << bit_index;
        }

        bitmap
    }

    /// Get common capabilities supported by this client
    fn get_common_capabilities(&self) -> Vec<u32> {
        // Don't advertise any capabilities for test-display-no-ssl
        vec![]
    }

    /// Get channel-specific capabilities
    fn get_channel_capabilities(&self) -> Vec<u32> {
        // No channel-specific capabilities for now - focus on basic connectivity
        vec![]
    }

    pub async fn handshake(&mut self) -> Result<()> {
        info!("=== SPICE Link Protocol Start ===");
        info!(
            "Channel type: {:?}, Channel ID: {}",
            self.channel_type, self.channel_id
        );

        // Validate connection_id according to protocol rules
        let connection_id = if self.channel_type == ChannelType::Main {
            // Main channel MUST use connection_id = 0
            if self.connection_id.is_some() && self.connection_id != Some(0) {
                return Err(SpiceError::Protocol(
                    "Main channel must use connection_id 0".to_string(),
                ));
            }
            info!("✓ Main channel using connection_id = 0 (new session)");
            0
        } else {
            // Non-main channels use the same connection_id as main channel (0 for new sessions)
            match self.connection_id {
                Some(id) => {
                    info!(
                        "✓ Non-main channel using connection_id = {} (0x{:08x})",
                        id, id
                    );
                    id
                }
                None => {
                    return Err(SpiceError::Protocol(
                        "Non-main channels must have connection_id set".to_string(),
                    ))
                }
            }
        };

        // Get capabilities
        let common_caps = self.get_common_capabilities();
        let channel_caps = self.get_channel_capabilities();

        info!("Common capabilities: {:?}", common_caps);
        info!("Channel capabilities: {:?}", channel_caps);

        // Encode capabilities as bitmaps
        let common_caps_bitmap = Self::encode_capabilities(&common_caps);
        let channel_caps_bitmap = Self::encode_capabilities(&channel_caps);

        info!("Common caps bitmap: {:?}", common_caps_bitmap);
        info!("Channel caps bitmap: {:?}", channel_caps_bitmap);

        // Create link message
        info!(
            "Creating SpiceLinkMess with connection_id={}, channel_type={}, channel_id={}",
            connection_id, self.channel_type as u8, self.channel_id
        );
        let link_mess = SpiceLinkMess {
            connection_id,
            channel_type: self.channel_type as u8,
            channel_id: self.channel_id,
            num_common_caps: common_caps_bitmap.len() as u32,
            num_channel_caps: channel_caps_bitmap.len() as u32,
            caps_offset: 20, // Actual serialized size with padding
        };

        // Use binrw for proper SPICE protocol serialization
        use binrw::BinWrite;
        let mut mess_cursor = std::io::Cursor::new(Vec::new());
        link_mess
            .write(&mut mess_cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write link message: {e}")))?;

        // Get the message bytes and append capabilities
        let mut mess_bytes = mess_cursor.into_inner();

        // Append capability bitmaps
        for cap_word in &common_caps_bitmap {
            mess_bytes.extend_from_slice(&cap_word.to_le_bytes());
        }
        for cap_word in &channel_caps_bitmap {
            mess_bytes.extend_from_slice(&cap_word.to_le_bytes());
        }

        // Send link header with the total size including capabilities
        let link_header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: mess_bytes.len() as u32, // Total size including capabilities
        };

        let mut header_cursor = std::io::Cursor::new(Vec::new());
        link_header
            .write(&mut header_cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write link header: {e}")))?;
        let header_bytes = header_cursor.into_inner();

        info!("Sending SPICE link header: {:?}", header_bytes);
        self.send_raw(&header_bytes).await?;

        info!(
            "Sending SPICE link message ({} bytes): {:?}",
            mess_bytes.len(),
            mess_bytes
        );
        self.send_raw(&mess_bytes).await?;

        // Read reply
        let reply_bytes = self.read_raw(std::mem::size_of::<SpiceLinkReply>()).await?;

        // Debug: log the raw bytes we received
        info!("Received reply bytes: {:?}", reply_bytes);
        if reply_bytes.len() >= 4 {
            let magic_bytes = &reply_bytes[0..4];
            let magic = u32::from_le_bytes([
                magic_bytes[0],
                magic_bytes[1],
                magic_bytes[2],
                magic_bytes[3],
            ]);
            info!(
                "Magic in reply: 0x{:08x}, expected: 0x{:08x}",
                magic, SPICE_MAGIC
            );
        }

        // Use binrw for proper SPICE protocol deserialization
        use binrw::BinRead;
        let mut cursor = std::io::Cursor::new(&reply_bytes);
        let reply = SpiceLinkReply::read(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to parse link reply: {e}")))?;

        if reply.magic != SPICE_MAGIC {
            return Err(SpiceError::Protocol(format!(
                "Invalid magic in reply: got 0x{:08x}, expected 0x{:08x}",
                reply.magic, SPICE_MAGIC
            )));
        }

        // Read the link message data if size > 0
        if reply.size > 0 {
            info!("Reading {} bytes of link message data", reply.size);
            let link_data = self.read_raw(reply.size as usize).await?;
            info!("Link message data: {:?}", link_data);

            // Parse the link reply data using binrw
            use binrw::BinRead;
            let mut data_cursor = std::io::Cursor::new(&link_data);
            let reply_data = SpiceLinkReplyData::read(&mut data_cursor).map_err(|e| {
                SpiceError::Protocol(format!("Failed to parse link reply data: {e}"))
            })?;

            info!(
                "Link reply: error={}, num_common_caps={}, num_channel_caps={}",
                reply_data.error, reply_data.num_common_caps, reply_data.num_channel_caps
            );

            if reply_data.error == 0 {
                // Server sent public key
                let pub_key_der = &reply_data.pub_key;
                info!("Server provided RSA public key (162 bytes)");

                // Check if we advertised AUTH_SELECTION capability
                let common_caps = self.get_common_capabilities();
                let advertised_auth_selection =
                    common_caps.contains(&SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION);

                if advertised_auth_selection {
                    // Send authentication mechanism selection
                    info!(
                        "Sending authentication mechanism selection (SPICE_COMMON_CAP_AUTH_SPICE)"
                    );
                    let auth_mechanism = SpiceLinkAuthMechanism {
                        auth_mechanism: SPICE_COMMON_CAP_AUTH_SPICE,
                    };

                    use binrw::BinWrite;
                    let mut auth_cursor = std::io::Cursor::new(Vec::new());
                    auth_mechanism.write(&mut auth_cursor).map_err(|e| {
                        SpiceError::Protocol(format!("Failed to write auth mechanism: {e}"))
                    })?;
                    let auth_bytes = auth_cursor.into_inner();
                    self.send_raw(&auth_bytes).await?;
                } else {
                    info!("Not sending auth mechanism (AUTH_SELECTION not advertised)");
                }

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
                        info!(
                            "Successfully encrypted password, sending {} bytes",
                            encrypted_password.len()
                        );
                        self.send_raw(&encrypted_password).await?;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to encrypt password: {}, sending zeros as fallback",
                            e
                        );
                        let zeros = vec![0u8; 128];
                        self.send_raw(&zeros).await?;
                    }
                }

                // Read link result after authentication
                info!("Reading link result after authentication");
                let link_result = self.read_raw(4).await?;
                let auth_error = u32::from_le_bytes([
                    link_result[0],
                    link_result[1],
                    link_result[2],
                    link_result[3],
                ]);

                if auth_error != 0 {
                    let error_name = match auth_error {
                        1 => "SPICE_LINK_ERR_ERROR",
                        2 => "SPICE_LINK_ERR_INVALID_MAGIC",
                        3 => "SPICE_LINK_ERR_INVALID_DATA",
                        4 => "SPICE_LINK_ERR_VERSION_MISMATCH",
                        5 => "SPICE_LINK_ERR_NEED_SECURED",
                        6 => "SPICE_LINK_ERR_NEED_UNSECURED",
                        7 => "SPICE_LINK_ERR_PERMISSION_DENIED",
                        8 => "SPICE_LINK_ERR_BAD_CONNECTION_ID",
                        9 => "SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE",
                        _ => "UNKNOWN_ERROR",
                    };
                    return Err(SpiceError::Protocol(format!(
                        "Authentication failed with error code: {} ({})",
                        auth_error, error_name
                    )));
                }
                info!("✓ Authentication successful - Link result is 0 (SPICE_LINK_ERR_OK)");
            } else {
                // Handle link error
                let error_name = match reply_data.error {
                    1 => "SPICE_LINK_ERR_ERROR",
                    2 => "SPICE_LINK_ERR_INVALID_MAGIC",
                    3 => "SPICE_LINK_ERR_INVALID_DATA",
                    4 => "SPICE_LINK_ERR_VERSION_MISMATCH",
                    5 => "SPICE_LINK_ERR_NEED_SECURED",
                    6 => "SPICE_LINK_ERR_NEED_UNSECURED",
                    7 => "SPICE_LINK_ERR_PERMISSION_DENIED",
                    8 => "SPICE_LINK_ERR_BAD_CONNECTION_ID",
                    9 => "SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE",
                    _ => "Unknown error",
                };
                return Err(SpiceError::Protocol(format!(
                    "Link stage failed with error code: {} ({})",
                    reply_data.error, error_name
                )));
            }
        }

        // Mark handshake as complete
        self.handshake_complete = true;
        info!("=== SPICE Link Protocol Complete ===");
        info!(
            "✓ Valid connection established for {:?} channel",
            self.channel_type
        );

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
                    websocket.send_with_u8_array(data).map_err(|e| {
                        SpiceError::Protocol(format!("Failed to send WebSocket data: {:?}", e))
                    })?;
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
            while attempts < 2000 {
                // Increased timeout for SPICE handshake
                if let Ok(mut buffer) = self.byte_buffer.lock() {
                    if buffer.len() >= len {
                        let data = buffer.drain(..len).collect();
                        info!("Read {} bytes from WebSocket: {:?}", len, data);
                        return Ok(data);
                    } else if !buffer.is_empty() {
                        if buffer.len() != last_buffer_size {
                            info!(
                                "Buffer has {} bytes, need {} (attempt {})",
                                buffer.len(),
                                len,
                                attempts
                            );
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
            info!(
                "Timeout after {} attempts waiting for {} bytes",
                attempts, len
            );
            Err(SpiceError::Protocol(
                "Timeout waiting for WebSocket data".to_string(),
            ))
        }
    }

    pub async fn read_message(&mut self) -> Result<(SpiceDataHeader, Vec<u8>)> {
        // SPICE protocol specifies exact sizes on the wire:
        // serial: 8 bytes, msg_type: 2 bytes, msg_size: 4 bytes, sub_list: 4 bytes = 18 bytes total
        const SPICE_DATA_HEADER_SIZE: usize = 18;

        let header_bytes = self.read_raw(SPICE_DATA_HEADER_SIZE).await?;

        debug!(
            "Raw header bytes ({}): {:?}",
            header_bytes.len(),
            header_bytes
        );

        use binrw::BinRead;
        let mut cursor = std::io::Cursor::new(&header_bytes);
        let header = SpiceDataHeader::read(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to parse data header: {e}")))?;

        debug!(
            "Parsed header: serial={}, type={}, size={}, sub_list={}",
            header.serial, header.msg_type, header.msg_size, header.sub_list
        );

        let data = self.read_raw(header.msg_size as usize).await?;

        Ok((header, data))
    }

    pub async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        // Use instance serial number tracking
        let serial = self.next_serial;
        self.next_serial += 1;

        debug!(
            "Sending message: type={}, size={}, serial={}",
            msg_type,
            data.len(),
            serial
        );

        let header = SpiceDataHeader {
            serial,
            msg_type,
            msg_size: data.len() as u32,
            sub_list: 0,
        };

        use binrw::BinWrite;
        let mut header_cursor = std::io::Cursor::new(Vec::new());
        header
            .write(&mut header_cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write data header: {e}")))?;
        let header_bytes = header_cursor.into_inner();

        info!(
            "Sending message: serial={}, type={}, size={}, header_size={}",
            serial,
            msg_type,
            data.len(),
            header_bytes.len()
        );
        self.send_raw(&header_bytes).await?;
        if !data.is_empty() {
            info!("Sending message data: {:?}", data);
            self.send_raw(data).await?;
        }

        Ok(())
    }
}
