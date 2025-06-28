use crate::channels::{Channel, ChannelConnection};
use crate::error::{Result, SpiceError};
use crate::protocol::*;
use tracing::{debug, error, info, warn};

pub struct MainChannel {
    connection: ChannelConnection,
}

impl MainChannel {
    pub async fn new(host: &str, port: u16) -> Result<Self> {
        let mut connection = ChannelConnection::new(host, port, ChannelType::Main, 0).await?;
        connection.handshake().await?;
        
        Ok(Self { connection })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(websocket_url: &str) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(websocket_url: &str, auth_token: Option<String>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Main, 0, auth_token).await?;
        connection.handshake().await?;
        
        Ok(Self { connection })
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_password(websocket_url: &str, auth_token: Option<String>, password: Option<String>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Main, 0, auth_token).await?;
        if let Some(password) = password {
            connection.set_password(password);
        }
        connection.handshake().await?;
        
        Ok(Self { connection })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Some SPICE servers send Init first, others expect client to request it
        info!("Trying to receive server init message or proceeding with client-initiated flow");
        
        // First, try to wait for a server-initiated message (WASM-compatible)
        #[cfg(target_arch = "wasm32")]
        {
            use gloo_timers::future::TimeoutFuture;
            
            let read_future = self.connection.read_message();
            let timeout_future = TimeoutFuture::new(2000); // Shorter timeout for fallback
            
            match tokio::select! {
                result = read_future => Some(result),
                _ = timeout_future => {
                    info!("No server init message received, trying client-initiated flow");
                    None
                }
            } {
                Some(Ok((header, data))) => {
                    info!("Received server message: type={}, size={}", header.msg_type, header.msg_size);
                    if header.msg_type == MainChannelMessage::Init as u16 {
                        info!("Received SpiceMsgMainInit from server");
                        self.handle_message(&header, &data).await?;
                    } else {
                        info!("Unexpected first message type: {}, handling anyway", header.msg_type);
                        self.handle_message(&header, &data).await?;
                    }
                }
                Some(Err(e)) => {
                    info!("Error waiting for server init: {}, trying client-initiated flow", e);
                    // Try client-initiated flow
                    self.try_client_initiated_flow().await?;
                }
                None => {
                    // Timeout - try client-initiated flow
                    self.try_client_initiated_flow().await?;
                }
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            match tokio::time::timeout(
                std::time::Duration::from_millis(2000),
                self.connection.read_message()
            ).await {
                Ok(Ok((header, data))) => {
                    info!("Received server message: type={}, size={}", header.msg_type, header.msg_size);
                    if header.msg_type == MainChannelMessage::Init as u16 {
                        info!("Received SpiceMsgMainInit from server");
                        self.handle_message(&header, &data).await?;
                    } else {
                        info!("Unexpected first message type: {}, handling anyway", header.msg_type);
                        self.handle_message(&header, &data).await?;
                    }
                }
                Ok(Err(e)) => {
                    info!("Error waiting for server init: {}, trying client-initiated flow", e);
                    self.try_client_initiated_flow().await?;
                }
                Err(_) => {
                    info!("No server init message received, trying client-initiated flow");
                    self.try_client_initiated_flow().await?;
                }
            }
        }
        
        info!("SPICE main channel ready");
        Ok(())
    }
    
    async fn try_client_initiated_flow(&mut self) -> Result<()> {
        // According to SPICE protocol, server should send SPICE_MSG_MAIN_INIT
        // after successful link handshake. Client just waits for it.
        info!("Waiting for server to send SPICE_MSG_MAIN_INIT");
        Ok(())
    }

    pub async fn get_channels_list(&mut self) -> Result<Vec<(ChannelType, u8)>> {
        // Request channels list
        info!("Requesting channels list (type {})", MainChannelMessage::ChannelsList as u16);
        self.connection.send_message(MainChannelMessage::ChannelsList as u16, &[]).await?;
        
        // Read response with timeout (WASM-compatible)
        #[cfg(target_arch = "wasm32")]
        {
            use gloo_timers::future::TimeoutFuture;
            
            let read_future = self.connection.read_message();
            let timeout_future = TimeoutFuture::new(3000);
            
            match tokio::select! {
                result = read_future => result,
                _ = timeout_future => {
                    info!("Channels list request timed out, returning default channels");
                    // Return default channels as fallback
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    debug!("Using default channels: {:?}", channels);
                    return Ok(channels);
                }
            } {
                Ok((header, data)) => {
                    info!("Channels list response: type={}, size={}, data={:?}", 
                          header.msg_type, header.msg_size, data);
                    
                    if header.msg_type != MainChannelMessage::ChannelsList as u16 {
                        info!("Expected channels list, got message type {}, using defaults", header.msg_type);
                        let mut channels = Vec::new();
                        channels.push((ChannelType::Display, 0));
                        channels.push((ChannelType::Inputs, 0));
                        channels.push((ChannelType::Cursor, 0));
                        return Ok(channels);
                    }

                    // Parse channels list (simplified - real implementation would parse binary data)
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    
                    debug!("Available channels: {:?}", channels);
                    Ok(channels)
                }
                Err(e) => {
                    info!("Error reading channels list: {}, using defaults", e);
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    Ok(channels)
                }
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            match tokio::time::timeout(
                std::time::Duration::from_millis(3000),
                self.connection.read_message()
            ).await {
                Ok(Ok((header, data))) => {
                    info!("Channels list response: type={}, size={}, data={:?}", 
                          header.msg_type, header.msg_size, data);
                    
                    if header.msg_type != MainChannelMessage::ChannelsList as u16 {
                        info!("Expected channels list, got message type {}, using defaults", header.msg_type);
                        let mut channels = Vec::new();
                        channels.push((ChannelType::Display, 0));
                        channels.push((ChannelType::Inputs, 0));
                        channels.push((ChannelType::Cursor, 0));
                        return Ok(channels);
                    }

                    // Parse channels list (simplified - real implementation would parse binary data)
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    
                    debug!("Available channels: {:?}", channels);
                    Ok(channels)
                }
                Ok(Err(e)) => {
                    info!("Error reading channels list: {}, using defaults", e);
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    Ok(channels)
                }
                Err(_) => {
                    info!("Channels list request timed out, returning default channels");
                    let mut channels = Vec::new();
                    channels.push((ChannelType::Display, 0));
                    channels.push((ChannelType::Inputs, 0));
                    channels.push((ChannelType::Cursor, 0));
                    debug!("Using default channels: {:?}", channels);
                    Ok(channels)
                }
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let (header, data) = self.connection.read_message().await?;
            self.handle_message(&header, &data).await?;
        }
    }
}

impl Channel for MainChannel {
    async fn handle_message(&mut self, header: &SpiceDataHeader, data: &[u8]) -> Result<()> {
        match header.msg_type {
            x if x == MainChannelMessage::Ping as u16 => {
                debug!("Received ping, sending pong");
                self.connection.send_message(MainChannelMessage::PingReply as u16, &[]).await?;
            }
            x if x == MainChannelMessage::PingReply as u16 => {
                debug!("Received ping reply");
            }
            x if x == MainChannelMessage::Init as u16 => {
                let init_msg: crate::protocol::SpiceMsgMainInit = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse Init: {}", e)))?;
                info!("Received SPICE_MSG_MAIN_INIT: session_id={}, display_hint={}, mouse_modes={:x}", 
                      init_msg.session_id, init_msg.display_channels_hint, init_msg.supported_mouse_modes);
                info!("Server mouse mode: {}, agent_connected: {}", 
                      init_msg.current_mouse_mode, init_msg.agent_connected);
                // TODO: Store init_msg data for use by the client
            }
            x if x == MainChannelMessage::ChannelsList as u16 => {
                debug!("Received channels list");
                // Already handled in handle_channels_list_message
            }
            x if x == MainChannelMessage::MouseMode as u16 => {
                let mouse_mode: SpiceMsgMainMouseMode = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse MouseMode: {}", e)))?;
                info!("Mouse mode changed to: {}", mouse_mode.mode);
                // TODO: Store mouse mode and notify input handling
            }
            x if x == MainChannelMessage::MultiMediaTime as u16 => {
                let mm_time: SpiceMsgMainMultiMediaTime = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse MultiMediaTime: {}", e)))?;
                debug!("Multimedia time: {}", mm_time.time);
                // TODO: Synchronize with multimedia time
            }
            x if x == MainChannelMessage::AgentConnected as u16 => {
                let agent_connected: SpiceMsgMainAgentConnected = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse AgentConnected: {}", e)))?;
                info!("Agent connected with error code: {}", agent_connected.error_code);
                // TODO: Initialize agent communication
            }
            x if x == MainChannelMessage::AgentDisconnected as u16 => {
                info!("Agent disconnected");
                // TODO: Clean up agent state
            }
            x if x == MainChannelMessage::AgentData as u16 => {
                let agent_data: SpiceMsgMainAgentData = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse AgentData: {}", e)))?;
                debug!("Received agent data: protocol {}, type {}, size {}", 
                       agent_data.protocol, agent_data.type_, agent_data.size);
                // TODO: Process agent data (clipboard, file transfer, etc.)
            }
            x if x == MainChannelMessage::AgentTokens as u16 => {
                let agent_tokens: SpiceMsgMainAgentTokens = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse AgentTokens: {}", e)))?;
                debug!("Agent tokens: {}", agent_tokens.num_tokens);
                // TODO: Update agent token count for flow control
            }
            x if x == MainChannelMessage::Notify as u16 => {
                let notify: SpiceMsgMainNotify = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse Notify: {}", e)))?;
                let message = String::from_utf8_lossy(&notify.message);
                match notify.severity {
                    0 => info!("Server info: {}", message),
                    1 => warn!("Server warning: {}", message),
                    2 => error!("Server error: {}", message),
                    _ => debug!("Server notification (severity {}): {}", notify.severity, message),
                }
            }
            x if x == MainChannelMessage::Disconnecting as u16 => {
                info!("Server is disconnecting");
                return Err(SpiceError::ConnectionClosed);
            }
            _ => {
                warn!("Unknown message type: {}", header.msg_type);
            }
        }
        
        Ok(())
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Main
    }
}