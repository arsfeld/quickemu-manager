use spice_client::{SpiceError, Result};
use spice_client::protocol::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Mock SPICE server for testing
pub struct MockSpiceServer {
    listener: TcpListener,
    connections: Arc<Mutex<Vec<MockConnection>>>,
    config: MockServerConfig,
}

#[derive(Clone)]
pub struct MockServerConfig {
    pub require_auth: bool,
    pub auth_token: Option<String>,
    pub reject_connections: bool,
    pub support_channels: Vec<ChannelType>,
    pub display_width: u32,
    pub display_height: u32,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            require_auth: false,
            auth_token: None,
            reject_connections: false,
            support_channels: vec![ChannelType::Main, ChannelType::Display],
            display_width: 1024,
            display_height: 768,
        }
    }
}

pub struct MockConnection {
    pub id: String,
    pub channels: Vec<ChannelType>,
}

impl MockSpiceServer {
    pub async fn new(config: MockServerConfig) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await
            .map_err(|e| SpiceError::Io(e))?;
            
        Ok(Self {
            listener,
            connections: Arc::new(Mutex::new(Vec::new())),
            config,
        })
    }
    
    pub fn port(&self) -> u16 {
        self.listener.local_addr()
            .map(|addr| addr.port())
            .unwrap_or(0)
    }
    
    pub async fn run(&self) -> Result<()> {
        loop {
            let (stream, addr) = self.listener.accept().await
                .map_err(|e| SpiceError::Io(e))?;
                
            println!("Mock server: Accepted connection from {}", addr);
            
            let config = self.config.clone();
            let connections = Arc::clone(&self.connections);
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, config, connections).await {
                    eprintln!("Mock server: Connection error: {:?}", e);
                }
            });
        }
    }
    
    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    config: MockServerConfig,
    connections: Arc<Mutex<Vec<MockConnection>>>,
) -> Result<()> {
    if config.reject_connections {
        return Ok(());
    }
    
    // According to SPICE protocol, we should read SpiceLinkHeader first
    let mut header_buf = vec![0u8; std::mem::size_of::<SpiceLinkHeader>()];
    stream.read_exact(&mut header_buf).await?;
    
    let header: SpiceLinkHeader = bincode::deserialize(&header_buf)
        .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize header: {}", e)))?;
        
    println!("Mock server: Received link header: {:?}", header);
    
    // Verify magic
    if header.magic != SPICE_MAGIC {
        return Err(SpiceError::Protocol("Invalid magic".to_string()));
    }
    
    // Read link message - the size in header tells us how much to read
    let mut mess_buf = vec![0u8; header.size as usize];
    println!("Mock server: Expecting {} bytes for link message", header.size);
    
    stream.read_exact(&mut mess_buf).await?;
    println!("Mock server: Read {} bytes for link message", mess_buf.len());
    
    let mess: SpiceLinkMess = bincode::deserialize(&mess_buf)
        .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize message: {}", e)))?;
        
    println!("Mock server: Received link message for channel type {} id {}", 
             mess.channel_type, mess.channel_id);
    
    // According to SPICE protocol, server sends SpiceLinkReply
    let reply = SpiceLinkReply {
        magic: SPICE_MAGIC,
        major_version: SPICE_VERSION_MAJOR,
        minor_version: SPICE_VERSION_MINOR,
        size: 0, // No capabilities data for now
    };
    
    let reply_bytes = bincode::serialize(&reply)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize reply: {}", e)))?;
        
    stream.write_all(&reply_bytes).await?;
    stream.flush().await?;
    
    println!("Mock server: Sent link reply");
    
    // Add to connections
    let connection = MockConnection {
        id: format!("conn-{}", connections.lock().unwrap().len()),
        channels: vec![ChannelType::from(mess.channel_type)],
    };
    connections.lock().unwrap().push(connection);
    
    // After handshake, handle channel-specific protocol
    match ChannelType::from(mess.channel_type) {
        ChannelType::Main => handle_main_channel(&mut stream, &config).await?,
        ChannelType::Display => handle_display_channel(&mut stream, &config).await?,
        _ => {
            println!("Mock server: Unsupported channel type {}", mess.channel_type);
        }
    }
    
    Ok(())
}

async fn handle_main_channel(stream: &mut TcpStream, _config: &MockServerConfig) -> Result<()> {
    // First serialize the init data to get the actual size
    let init_data = SpiceMsgMainInit {
        session_id: 1,
        display_channels_hint: 1,
        supported_mouse_modes: 0x01 | 0x02, // Client and server modes
        current_mouse_mode: 0x02, // Server mode
        agent_connected: 0,
        agent_tokens: 0,
        multi_media_time: 0,
        ram_hint: 0,
    };
    
    let init_data_bytes = bincode::serialize(&init_data)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize init data: {}", e)))?;
    
    // Create header with actual serialized data size
    let init_msg = SpiceDataHeader {
        serial: 1,
        msg_type: SPICE_MSG_MAIN_INIT,
        msg_size: init_data_bytes.len() as u32,
        sub_list: 0,
    };
    
    let header_bytes = bincode::serialize(&init_msg)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize init header: {}", e)))?;
    
    // Pad header to expected size (client expects 24 bytes)
    let mut padded_header = vec![0u8; 24];
    padded_header[..header_bytes.len()].copy_from_slice(&header_bytes);
    
    // Write padded header then data
    stream.write_all(&padded_header).await?;
    stream.write_all(&init_data_bytes).await?;
    stream.flush().await?;
    
    println!("Mock server: Sent main init message");
    
    // The client might request channels list or we can send it proactively
    // For now, keep connection alive and handle any incoming messages
    loop {
        let mut header_buf = vec![0u8; std::mem::size_of::<SpiceDataHeader>()];
        match stream.read_exact(&mut header_buf).await {
            Ok(_) => {
                match bincode::deserialize::<SpiceDataHeader>(&header_buf) {
                    Ok(header) => {
                        println!("Mock server: Received message type {} on main channel", header.msg_type);
                        
                        // Read message data if any
                        if header.msg_size > 0 {
                            let mut data_buf = vec![0u8; header.msg_size as usize];
                            stream.read_exact(&mut data_buf).await?;
                        }
                        
                        // Handle specific message types
                        match header.msg_type {
                            x if x == MainChannelMessage::Ping as u16 => {
                                // Send pong
                                let pong = SpiceDataHeader {
                                    serial: header.serial + 1,
                                    msg_type: MainChannelMessage::PingReply as u16,
                                    msg_size: 0,
                                    sub_list: 0,
                                };
                                let pong_bytes = bincode::serialize(&pong)
                                    .map_err(|e| SpiceError::Protocol(format!("Failed to serialize pong: {}", e)))?;
                                
                                // Pad to 24 bytes
                                let mut padded_pong = vec![0u8; 24];
                                padded_pong[..pong_bytes.len()].copy_from_slice(&pong_bytes);
                                
                                stream.write_all(&padded_pong).await?;
                                stream.flush().await?;
                                println!("Mock server: Sent pong reply");
                            }
                            x if x == MainChannelMessage::ChannelsList as u16 => {
                                // Send channels list reply
                                // In a real implementation, this would contain the actual channel data
                                // For now, send an empty list (just the header)
                                let channels_reply = SpiceDataHeader {
                                    serial: header.serial + 1,
                                    msg_type: MainChannelMessage::ChannelsList as u16,
                                    msg_size: 0, // Empty channels list for simplicity
                                    sub_list: 0,
                                };
                                let reply_bytes = bincode::serialize(&channels_reply)
                                    .map_err(|e| SpiceError::Protocol(format!("Failed to serialize channels list: {}", e)))?;
                                
                                // Pad to 24 bytes
                                let mut padded_reply = vec![0u8; 24];
                                padded_reply[..reply_bytes.len()].copy_from_slice(&reply_bytes);
                                
                                stream.write_all(&padded_reply).await?;
                                stream.flush().await?;
                                println!("Mock server: Sent channels list reply");
                            }
                            _ => {
                                println!("Mock server: Unhandled message type {}", header.msg_type);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Mock server: Failed to deserialize header: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                println!("Mock server: Connection closed or error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

async fn handle_display_channel(stream: &mut TcpStream, config: &MockServerConfig) -> Result<()> {
    // Send display mode message
    let mode_msg = SpiceDataHeader {
        serial: 1,
        msg_type: SPICE_MSG_DISPLAY_MODE,
        msg_size: 8, // width + height
        sub_list: 0,
    };
    
    let header_bytes = bincode::serialize(&mode_msg)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize mode header: {}", e)))?;
        
    stream.write_all(&header_bytes).await?;
    
    // Send mode data (simplified)
    stream.write_all(&config.display_width.to_le_bytes()).await?;
    stream.write_all(&config.display_height.to_le_bytes()).await?;
    stream.flush().await?;
    
    println!("Mock server: Sent display mode {}x{}", config.display_width, config.display_height);
    
    // Keep connection alive
    loop {
        let mut buf = vec![0u8; 1024];
        match stream.read(&mut buf).await {
            Ok(0) => break, // Connection closed
            Ok(n) => {
                println!("Mock server: Received {} bytes on display channel", n);
            }
            Err(e) => {
                println!("Mock server: Read error on display channel: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spice_client::SpiceClient;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_mock_server_basic() -> Result<()> {
        let config = MockServerConfig::default();
        let server = MockSpiceServer::new(config).await?;
        let port = server.port();
        
        assert!(port > 0);
        assert_eq!(server.connection_count(), 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_protocol_sizes() -> Result<()> {
        // Test that our struct sizes match expectations
        println!("SpiceLinkHeader size: {}", std::mem::size_of::<SpiceLinkHeader>());
        println!("SpiceLinkMess size: {}", std::mem::size_of::<SpiceLinkMess>());
        println!("SpiceLinkReply size: {}", std::mem::size_of::<SpiceLinkReply>());
        println!("SpiceDataHeader size: {}", std::mem::size_of::<SpiceDataHeader>());
        
        // Test serialization
        let header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 20,
        };
        
        let serialized = bincode::serialize(&header)
            .map_err(|e| SpiceError::Protocol(format!("Serialize error: {}", e)))?;
        println!("Serialized SpiceLinkHeader size: {}, bytes: {:?}", serialized.len(), serialized);
        
        // Test SpiceDataHeader serialization
        let data_header = SpiceDataHeader {
            serial: 1,
            msg_type: 103,
            msg_size: 32,
            sub_list: 0,
        };
        
        let data_serialized = bincode::serialize(&data_header)
            .map_err(|e| SpiceError::Protocol(format!("Serialize error: {}", e)))?;
        println!("Serialized SpiceDataHeader size: {}, bytes: {:?}", data_serialized.len(), data_serialized);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_mock_server_connection() -> Result<()> {
        // Initialize tracing for debugging
        let _ = tracing_subscriber::fmt()
            .with_env_filter("spice_client=trace")
            .try_init();
            
        let config = MockServerConfig::default();
        let server = Arc::new(MockSpiceServer::new(config).await?);
        let port = server.port();
        
        // Run server in background
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let _ = server_clone.run().await;
        });
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Connect client
        let mut client = SpiceClient::new("127.0.0.1".to_string(), port);
        
        // Connect with timeout
        let connect_result = timeout(Duration::from_secs(2), client.connect()).await;
        
        match connect_result {
            Ok(Ok(_)) => {
                // Start event loop in background
                let client_handle = tokio::spawn(async move {
                    let _ = client.start_event_loop().await;
                });
                
                // Give it time to process messages
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                // Check that we connected
                assert!(server.connection_count() > 0);
                
                // Abort both tasks
                client_handle.abort();
                server_handle.abort();
                
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SpiceError::Protocol("Connection timeout".to_string())),
        }
    }
}