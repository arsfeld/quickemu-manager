use crate::error::{Result, SpiceError};
use crate::protocol::*;
use crate::transport::{Transport, TransportConfig, create_transport};
use crate::wire_format::{read_message, write_message};
use bincode::Options;
use bytes::BytesMut;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Oaep, RsaPublicKey};
use rsa::rand_core::OsRng;
use sha1::Sha1;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// A connection to a SPICE channel
pub struct ChannelConnection {
    transport: Box<dyn Transport>,
    channel_type: ChannelType,
    pub channel_id: u8,
    password: Option<String>,
    connection_id: Option<u32>,
    next_serial: u64,
    handshake_complete: bool,
}

/// Encrypt a password using RSA-OAEP with SHA-1
fn encrypt_password(password: &str, pub_key_der: &[u8]) -> Result<Vec<u8>> {
    match RsaPublicKey::from_public_key_der(pub_key_der) {
        Ok(public_key) => {
            let padding = Oaep::new::<Sha1>();
            match public_key.encrypt(&mut OsRng, padding, password.as_bytes()) {
                Ok(encrypted) => Ok(encrypted),
                Err(e) => Err(SpiceError::Protocol(format!("Failed to encrypt password: {}", e)))
            }
        }
        Err(e) => {
            warn!("Failed to parse RSA public key: {}, trying raw modulus/exponent", e);
            Err(SpiceError::Protocol(format!("Failed to parse RSA public key: {}", e)))
        }
    }
}

impl ChannelConnection {
    /// Create a new channel connection
    pub async fn new(
        host: &str,
        port: u16,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        let config = TransportConfig {
            host: host.to_string(),
            port,
            #[cfg(target_arch = "wasm32")]
            websocket_url: None,
            #[cfg(target_arch = "wasm32")]
            auth_token: None,
        };
        
        let transport = create_transport(config).await?;
        
        Ok(Self {
            transport,
            channel_type,
            channel_id,
            password: None,
            connection_id: None,
            next_serial: 1,
            handshake_complete: false,
        })
    }
    
    /// Create a new channel connection with WebSocket URL (WASM)
    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(
        websocket_url: &str,
        channel_type: ChannelType,
        channel_id: u8,
    ) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_type, channel_id, None).await
    }
    
    /// Create a new channel connection with WebSocket URL and auth token (WASM)
    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(
        websocket_url: &str,
        channel_type: ChannelType,
        channel_id: u8,
        auth_token: Option<String>,
    ) -> Result<Self> {
        let config = TransportConfig {
            host: String::new(), // Not used for WebSocket
            port: 0, // Not used for WebSocket
            websocket_url: Some(websocket_url.to_string()),
            auth_token,
        };
        
        let transport = create_transport(config).await?;
        
        Ok(Self {
            transport,
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

    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    pub async fn handshake(&mut self) -> Result<()> {
        if self.handshake_complete {
            return Ok(());
        }

        info!("Starting SPICE handshake for {:?} channel {}", self.channel_type, self.channel_id);

        // Send RedLinkMess
        self.send_link_message().await?;

        // Wait for RedLinkReply
        let reply_data = self.wait_for_link_reply().await?;

        // Send authentication if needed
        if self.password.is_some() {
            self.send_auth(&reply_data).await?;
        }

        self.handshake_complete = true;
        info!("SPICE handshake completed for {:?} channel {}", self.channel_type, self.channel_id);

        Ok(())
    }

    async fn send_link_message(&mut self) -> Result<()> {
        // Prepare capabilities
        let mut common_caps: Vec<u32> = vec![];
        let mut channel_caps: Vec<u32> = vec![];
        
        // Common capabilities for all channels
        // TODO: Add SPICE_COMMON_CAP_MINI_HEADER support once we implement 6-byte header handling
        // TODO: test-display-no-ssl may not support AUTH_SELECTION, try without capabilities
        // let common_cap_bits = 1 << SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION;
        // debug!("Common capability bits: PROTOCOL_AUTH_SELECTION={}, combined={}",
        //        SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION, common_cap_bits);
        // common_caps.push(common_cap_bits);
        
        // Channel-specific capabilities
        match self.channel_type {
            ChannelType::Main => {
                channel_caps.push(1 << SPICE_MAIN_CAP_AGENT_CONNECTED_TOKENS);
            }
            ChannelType::Display => {
                channel_caps.push(
                    (1 << SPICE_DISPLAY_CAP_SIZED_STREAM) |
                    (1 << SPICE_DISPLAY_CAP_STREAM_REPORT) |
                    (1 << SPICE_DISPLAY_CAP_MULTI_CODEC) |
                    (1 << SPICE_DISPLAY_CAP_CODEC_MJPEG)
                );
            }
            _ => {}
        }
        
        // Calculate message size
        let message_size = 20 + (common_caps.len() + channel_caps.len()) * 4;
        
        let header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: message_size as u32,
        };

        let link_mess = SpiceLinkMess {
            connection_id: self.connection_id.unwrap_or(0),
            channel_type: self.channel_type as u8,
            channel_id: self.channel_id,
            num_common_caps: common_caps.len() as u32,
            num_channel_caps: channel_caps.len() as u32,
            caps_offset: 20, // Offset where capabilities start (after the struct)
        };

        // Serialize and send header
        let header_bytes = bincode::DefaultOptions::new()
            .with_little_endian()
            .with_fixint_encoding()
            .serialize(&header)?;
        
        self.transport.write_all(&header_bytes).await
            .map_err(|e| SpiceError::Io(e))?;

        // Serialize and send link message
        let link_bytes = bincode::DefaultOptions::new()
            .with_little_endian()
            .with_fixint_encoding()
            .serialize(&link_mess)?;
        
        self.transport.write_all(&link_bytes).await
            .map_err(|e| SpiceError::Io(e))?;
        
        // Send capabilities
        for cap in &common_caps {
            let cap_bytes = cap.to_le_bytes();
            self.transport.write_all(&cap_bytes).await
                .map_err(|e| SpiceError::Io(e))?;
        }
        
        for cap in &channel_caps {
            let cap_bytes = cap.to_le_bytes();
            self.transport.write_all(&cap_bytes).await
                .map_err(|e| SpiceError::Io(e))?;
        }

        debug!("Sent SpiceLinkMess with {} common caps and {} channel caps for {:?} channel {}", 
               common_caps.len(), channel_caps.len(), self.channel_type, self.channel_id);
        Ok(())
    }

    async fn wait_for_link_reply(&mut self) -> Result<Vec<u8>> {
        let mut header_buf = vec![0u8; 16];
        
        // Read the header
        let mut total_read = 0;
        while total_read < 16 {
            let n = self.transport.read(&mut header_buf[total_read..]).await
                .map_err(|e| SpiceError::Io(e))?;
            if n == 0 {
                return Err(SpiceError::Protocol("Connection closed while reading link reply header".to_string()));
            }
            total_read += n;
        }

        let header: SpiceLinkHeader = bincode::DefaultOptions::new()
            .with_little_endian()
            .with_fixint_encoding()
            .deserialize(&header_buf)?;

        if header.magic != SPICE_MAGIC {
            return Err(SpiceError::Protocol(format!("Invalid magic in link reply: {:#x}", header.magic)));
        }

        // Read the reply data
        let mut reply_data = vec![0u8; header.size as usize];
        total_read = 0;
        while total_read < header.size as usize {
            let n = self.transport.read(&mut reply_data[total_read..]).await
                .map_err(|e| SpiceError::Io(e))?;
            if n == 0 {
                return Err(SpiceError::Protocol("Connection closed while reading link reply data".to_string()));
            }
            total_read += n;
        }

        debug!("Received SpiceLinkReply, size: {}", header.size);
        Ok(reply_data)
    }

    async fn send_auth(&mut self, reply_data: &[u8]) -> Result<()> {
        if reply_data.len() < 174 {
            return Err(SpiceError::Protocol("Link reply too short for auth".to_string()));
        }

        let error_code = u32::from_le_bytes([reply_data[0], reply_data[1], reply_data[2], reply_data[3]]);
        if error_code != 0 {
            return Err(SpiceError::Protocol(format!("Server returned error: {}", error_code)));
        }

        let pub_key = &reply_data[4..166];
        let password = self.password.as_ref().unwrap();
        
        info!("Encrypting password for authentication");
        let encrypted = encrypt_password(password, pub_key)?;

        self.transport.write_all(&encrypted).await
            .map_err(|e| SpiceError::Io(e))?;

        debug!("Sent encrypted password");
        Ok(())
    }

    pub async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        let header = SpiceDataHeader {
            serial: self.next_serial,
            msg_type,
            msg_size: data.len() as u32,
            sub_list: 0,
        };
        self.next_serial += 1;

        write_message(&mut self.transport, &header, data).await
    }

    pub async fn read_message(&mut self) -> Result<(SpiceDataHeader, Vec<u8>)> {
        read_message(&mut self.transport).await
    }

    pub async fn read_message_with_timeout(&mut self, timeout: Duration) -> Result<(SpiceDataHeader, Vec<u8>)> {
        // For now, just call read_message
        // TODO: Implement actual timeout logic
        self.read_message().await
    }
}