//! Test utilities for SPICE client

use crate::error::Result;
use crate::protocol::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

/// Mock SPICE server for testing
#[derive(Clone)]
pub struct MockSpiceServer {
    addr: SocketAddr,
    connections: Arc<Mutex<HashMap<u8, TcpStream>>>,
}

impl MockSpiceServer {
    pub async fn new(bind_addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let addr = listener.local_addr()?;
        
        // Start accepting connections in background
        let connections = Arc::new(Mutex::new(HashMap::new()));
        let connections_clone = connections.clone();
        
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let connections = connections_clone.clone();
                    tokio::spawn(async move {
                        // Handle handshake
                        if let Ok(_) = handle_handshake(&mut stream).await {
                            // Store connection by channel ID (simplified)
                            let mut conns = connections.lock().await;
                            let channel_id = conns.len() as u8;
                            conns.insert(channel_id, stream);
                        }
                    });
                }
            }
        });
        
        Ok(Self {
            addr,
            connections,
        })
    }
    
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
    
    pub async fn send_display_message(&self, msg_type: u16, data: &impl serde::Serialize) -> Result<()> {
        self.send_message_to_channel(0, msg_type, data).await
    }
    
    pub async fn send_cursor_message(&self, msg_type: u16, data: &impl serde::Serialize) -> Result<()> {
        self.send_message_to_channel(0, msg_type, data).await
    }
    
    pub async fn send_display_message_to_channel(&self, channel_id: u8, msg_type: u16, data: &impl serde::Serialize) -> Result<()> {
        self.send_message_to_channel(channel_id, msg_type, data).await
    }
    
    async fn send_message_to_channel(&self, channel_id: u8, msg_type: u16, data: &impl serde::Serialize) -> Result<()> {
        let mut connections = self.connections.lock().await;
        if let Some(stream) = connections.get_mut(&channel_id) {
            let data_bytes = bincode::serialize(data)
                .map_err(|e| crate::error::SpiceError::Protocol(format!("Serialize error: {}", e)))?;
            
            let header = SpiceDataHeader {
                serial: 1,
                msg_type,
                msg_size: data_bytes.len() as u32,
                sub_list: 0,
            };
            
            let header_bytes = bincode::serialize(&header)
                .map_err(|e| crate::error::SpiceError::Protocol(format!("Serialize error: {}", e)))?;
            let mut padded_header = vec![0u8; 24];
            padded_header[..header_bytes.len()].copy_from_slice(&header_bytes);
            
            stream.write_all(&padded_header).await?;
            stream.write_all(&data_bytes).await?;
            stream.flush().await?;
        }
        
        Ok(())
    }
}

async fn handle_handshake(stream: &mut TcpStream) -> Result<()> {
    // Read link header
    let mut header_buf = vec![0u8; std::mem::size_of::<SpiceLinkHeader>()];
    stream.read_exact(&mut header_buf).await?;
    
    // Read link message
    let header: SpiceLinkHeader = bincode::deserialize(&header_buf)
        .map_err(|e| crate::error::SpiceError::Protocol(format!("Deserialize error: {}", e)))?;
    let mut mess_buf = vec![0u8; header.size as usize];
    stream.read_exact(&mut mess_buf).await?;
    
    // Send reply
    let reply = SpiceLinkReply {
        magic: SPICE_MAGIC,
        major_version: SPICE_VERSION_MAJOR,
        minor_version: SPICE_VERSION_MINOR,
        size: 0,
    };
    
    let reply_bytes = bincode::serialize(&reply)
        .map_err(|e| crate::error::SpiceError::Protocol(format!("Serialize error: {}", e)))?;
    stream.write_all(&reply_bytes).await?;
    stream.flush().await?;
    
    Ok(())
}