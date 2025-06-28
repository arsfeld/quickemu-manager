use crate::channels::{Channel, ChannelConnection};
use crate::error::{Result, SpiceError};
use crate::protocol::*;
use tracing::{debug, info, warn};

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
        // Some SPICE implementations expect client to send a ping or init first
        info!("Trying ping to wake up server");
        
        match self.connection.send_message(MainChannelMessage::Ping as u16, &[]).await {
            Ok(()) => {
                info!("Sent ping to server");
                // Try to read response
                #[cfg(target_arch = "wasm32")]
                {
                    use gloo_timers::future::TimeoutFuture;
                    let read_future = self.connection.read_message();
                    let timeout_future = TimeoutFuture::new(1000);
                    
                    match tokio::select! {
                        result = read_future => result,
                        _ = timeout_future => {
                            info!("No ping response, server may not support ping");
                            return Ok(());
                        }
                    } {
                        Ok((header, data)) => {
                            info!("Received ping response: type={}, size={}", header.msg_type, header.msg_size);
                            self.handle_message(&header, &data).await?;
                        }
                        Err(e) => {
                            info!("Ping response error: {}, continuing anyway", e);
                        }
                    }
                }
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match tokio::time::timeout(
                        std::time::Duration::from_millis(1000),
                        self.connection.read_message()
                    ).await {
                        Ok(Ok((header, data))) => {
                            info!("Received ping response: type={}, size={}", header.msg_type, header.msg_size);
                            self.handle_message(&header, &data).await?;
                        }
                        Ok(Err(e)) => {
                            info!("Ping response error: {}, continuing anyway", e);
                        }
                        Err(_) => {
                            info!("No ping response, server may not support ping");
                        }
                    }
                }
            }
            Err(e) => {
                info!("Failed to send ping: {}, continuing anyway", e);
            }
        }
        
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
                debug!("Received init message");
                // Handle initialization
            }
            x if x == MainChannelMessage::ChannelsList as u16 => {
                debug!("Received channels list");
                // Handle channels list
            }
            x if x == MainChannelMessage::Notify as u16 => {
                debug!("Received notification");
                // Handle notification
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