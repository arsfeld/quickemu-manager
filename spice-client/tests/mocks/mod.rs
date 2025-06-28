use async_trait::async_trait;
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
    
    // Read link header
    let mut header_buf = vec![0u8; std::mem::size_of::<SpiceLinkHeader>()];
    stream.read_exact(&mut header_buf).await?;
    
    let header: SpiceLinkHeader = bincode::deserialize(&header_buf)
        .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize header: {}", e)))?;
        
    println!("Mock server: Received link header: {:?}", header);
    
    // Verify magic
    if header.magic != SPICE_MAGIC {
        return Err(SpiceError::Protocol("Invalid magic".to_string()));
    }
    
    // Read link message
    let mut mess_buf = vec![0u8; header.size as usize];
    stream.read_exact(&mut mess_buf).await?;
    
    let mess: SpiceLinkMess = bincode::deserialize(&mess_buf)
        .map_err(|e| SpiceError::Protocol(format!("Failed to deserialize message: {}", e)))?;
        
    println!("Mock server: Received link message for channel type {} id {}", 
             mess.channel_type, mess.channel_id);
    
    // Send link reply
    let reply = SpiceLinkReply {
        magic: SPICE_MAGIC,
        major_version: SPICE_VERSION_MAJOR,
        minor_version: SPICE_VERSION_MINOR,
        size: 0, // No additional data
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
    
    // Handle channel-specific messages
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
    // Send main init message
    let init_msg = SpiceDataHeader {
        serial: 1,
        msg_type: SPICE_MSG_MAIN_INIT,
        msg_size: 0,
        sub_list: 0,
    };
    
    let header_bytes = bincode::serialize(&init_msg)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize init header: {}", e)))?;
        
    stream.write_all(&header_bytes).await?;
    stream.flush().await?;
    
    println!("Mock server: Sent main init message");
    
    // Keep connection alive
    loop {
        let mut buf = vec![0u8; 1024];
        match stream.read(&mut buf).await {
            Ok(0) => break, // Connection closed
            Ok(n) => {
                println!("Mock server: Received {} bytes on main channel", n);
            }
            Err(e) => {
                println!("Mock server: Read error on main channel: {}", e);
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
    async fn test_mock_server_connection() -> Result<()> {
        let config = MockServerConfig::default();
        let server = Arc::new(MockSpiceServer::new(config).await?);
        let port = server.port();
        
        // Run server in background
        let server_clone = Arc::clone(&server);
        let server_handle = tokio::spawn(async move {
            let _ = server_clone.run().await;
        });
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Connect client
        let mut client = SpiceClient::new("127.0.0.1".to_string(), port);
        
        let connect_result = timeout(Duration::from_secs(2), client.connect()).await;
        
        // Abort server
        server_handle.abort();
        
        // Check result
        match connect_result {
            Ok(Ok(_)) => {
                assert!(server.connection_count() > 0);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SpiceError::Protocol("Connection timeout".to_string())),
        }
    }
}