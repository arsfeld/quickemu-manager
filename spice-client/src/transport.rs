use crate::error::Result;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use std::io;

/// Trait for abstracting network transport (TCP vs WebSocket)
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait Transport: Send + Sync {
    /// Read data from the transport
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    
    /// Write data to the transport
    async fn write(&mut self, buf: &[u8]) -> io::Result<()>;
    
    /// Write all data to the transport
    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
    
    /// Flush any buffered data
    async fn flush(&mut self) -> io::Result<()>;
    
    /// Check if the transport is connected
    fn is_connected(&self) -> bool;
    
    /// Close the transport
    async fn close(&mut self) -> io::Result<()>;
}

/// Trait for abstracting network transport (TCP vs WebSocket) - WASM version
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Transport {
    /// Read data from the transport
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    
    /// Write data to the transport
    async fn write(&mut self, buf: &[u8]) -> io::Result<()>;
    
    /// Write all data to the transport
    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
    
    /// Flush any buffered data
    async fn flush(&mut self) -> io::Result<()>;
    
    /// Check if the transport is connected
    fn is_connected(&self) -> bool;
    
    /// Close the transport
    async fn close(&mut self) -> io::Result<()>;
}

/// Transport configuration
pub struct TransportConfig {
    pub host: String,
    pub port: u16,
    #[cfg(target_arch = "wasm32")]
    pub websocket_url: Option<String>,
    #[cfg(target_arch = "wasm32")]
    pub auth_token: Option<String>,
}

/// Create a transport based on the target platform
pub async fn create_transport(config: TransportConfig) -> Result<Box<dyn Transport>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tcp::create_tcp_transport(&config.host, config.port).await
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        websocket::create_websocket_transport(config).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod tcp;

#[cfg(target_arch = "wasm32")]
pub mod websocket;