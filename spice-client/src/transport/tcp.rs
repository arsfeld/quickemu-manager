use super::Transport;
use crate::error::{Result, SpiceError};
use async_trait::async_trait;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

/// TCP transport implementation for native platforms
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf).await
    }

    async fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stream.write_all(buf).await?;
        Ok(())
    }

    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stream.write_all(buf).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        self.stream.flush().await
    }

    fn is_connected(&self) -> bool {
        // TCP doesn't have a simple way to check connection status
        // We'll assume it's connected until we get an error
        true
    }

    async fn close(&mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }
}

/// Create a TCP transport
pub async fn create_tcp_transport(host: &str, port: u16) -> Result<Box<dyn Transport>> {
    info!("Connecting to SPICE server at {}:{}", host, port);

    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|e| SpiceError::Connection(format!("Failed to connect: {}", e)))?;

    // Disable Nagle's algorithm for better latency
    stream
        .set_nodelay(true)
        .map_err(|e| SpiceError::Connection(format!("Failed to set TCP_NODELAY: {}", e)))?;

    info!("TCP connection established to {}:{}", host, port);

    Ok(Box::new(TcpTransport::new(stream)))
}
