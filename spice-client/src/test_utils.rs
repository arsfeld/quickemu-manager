//! Test utilities for SPICE client

use crate::error::Result;
use crate::protocol::*;
use binrw::io::Cursor;
use binrw::{BinRead, BinWrite};
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

        Ok(Self { addr, connections })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn send_display_message(&self, msg_type: u16, data_bytes: Vec<u8>) -> Result<()> {
        self.send_message_to_channel(0, msg_type, data_bytes).await
    }

    pub async fn send_cursor_message(&self, msg_type: u16, data_bytes: Vec<u8>) -> Result<()> {
        self.send_message_to_channel(0, msg_type, data_bytes).await
    }

    pub async fn send_display_message_to_channel(
        &self,
        channel_id: u8,
        msg_type: u16,
        data_bytes: Vec<u8>,
    ) -> Result<()> {
        self.send_message_to_channel(channel_id, msg_type, data_bytes)
            .await
    }

    async fn send_message_to_channel(
        &self,
        channel_id: u8,
        msg_type: u16,
        data_bytes: Vec<u8>,
    ) -> Result<()> {
        let mut connections = self.connections.lock().await;
        if let Some(stream) = connections.get_mut(&channel_id) {
            let header = SpiceDataHeader {
                serial: 1,
                msg_type,
                msg_size: data_bytes.len() as u32,
                sub_list: 0,
            };

            let mut header_bytes = Vec::new();
            header.write_le(&mut Cursor::new(&mut header_bytes))?;

            stream.write_all(&header_bytes).await?;
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
    let mut cursor = Cursor::new(&header_buf);
    let header = SpiceLinkHeader::read_le(&mut cursor)?;
    let mut mess_buf = vec![0u8; header.size as usize];
    stream.read_exact(&mut mess_buf).await?;

    // Send reply
    let reply = SpiceLinkReply {
        magic: SPICE_MAGIC,
        major_version: SPICE_VERSION_MAJOR,
        minor_version: SPICE_VERSION_MINOR,
        size: 0,
    };

    let mut reply_bytes = Vec::new();
    reply.write_le(&mut Cursor::new(&mut reply_bytes))?;
    stream.write_all(&reply_bytes).await?;
    stream.flush().await?;

    Ok(())
}
