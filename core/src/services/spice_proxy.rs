use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Configuration for the SPICE WebSocket proxy
#[derive(Debug, Clone)]
pub struct SpiceProxyConfig {
    pub bind_address: String,
    pub port_range: (u16, u16),
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub auth_timeout: Duration,
    pub enable_tls: bool,
}

impl Default for SpiceProxyConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port_range: (6080, 6089),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            auth_timeout: Duration::from_secs(10),
            enable_tls: false,
        }
    }
}

/// Information about an active SPICE console connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceConnection {
    pub vm_id: String,
    pub spice_port: u16,
    pub websocket_port: u16,
    pub auth_token: String,
    pub created_at: SystemTime,
    pub status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Authenticating,
    Connected,
    Disconnected,
    Error(String),
}

/// Console information returned to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleInfo {
    pub websocket_url: String,
    pub auth_token: String,
    pub connection_id: String,
}

/// SPICE WebSocket proxy service
pub struct SpiceProxyService {
    config: SpiceProxyConfig,
    connections: Arc<RwLock<HashMap<String, SpiceConnection>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl SpiceProxyService {
    pub fn new(config: SpiceProxyConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Start the SPICE proxy service
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let connections = self.connections.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::run_proxy_server(config, connections, &mut shutdown_rx).await {
                error!("SPICE proxy server error: {}", e);
            }
        });

        info!("SPICE WebSocket proxy service started");
        Ok(())
    }

    /// Stop the SPICE proxy service
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Clean up all connections
        self.connections.write().await.clear();
        
        info!("SPICE WebSocket proxy service stopped");
        Ok(())
    }

    /// Create a new console session for a VM
    pub async fn create_console_session(
        &self,
        vm_id: String,
        spice_port: u16,
    ) -> Result<ConsoleInfo> {
        // Generate secure authentication token
        let auth_token = Self::generate_auth_token();
        
        // Find available WebSocket port
        let websocket_port = self.find_available_port().await?;
        
        let connection = SpiceConnection {
            vm_id: vm_id.clone(),
            spice_port,
            websocket_port,
            auth_token: auth_token.clone(),
            created_at: SystemTime::now(),
            status: ConnectionStatus::Authenticating,
        };

        // Store connection
        let connection_id = format!("{}:{}", vm_id, websocket_port);
        self.connections
            .write()
            .await
            .insert(connection_id.clone(), connection);

        let console_info = ConsoleInfo {
            websocket_url: format!("ws://{}:{}", self.config.bind_address, websocket_port),
            auth_token,
            connection_id,
        };

        info!(
            "Created console session for VM '{}' on port {}",
            vm_id, websocket_port
        );

        Ok(console_info)
    }

    /// Remove a console session
    pub async fn remove_console_session(&self, connection_id: &str) -> Result<()> {
        if self.connections.write().await.remove(connection_id).is_some() {
            info!("Removed console session: {}", connection_id);
        }
        Ok(())
    }

    /// Get status of a console session
    pub async fn get_console_status(&self, connection_id: &str) -> Option<ConnectionStatus> {
        self.connections
            .read()
            .await
            .get(connection_id)
            .map(|conn| conn.status.clone())
    }

    /// List all active console connections
    pub async fn list_connections(&self) -> Vec<SpiceConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    /// Main proxy server loop
    async fn run_proxy_server(
        config: SpiceProxyConfig,
        connections: Arc<RwLock<HashMap<String, SpiceConnection>>>,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) -> Result<()> {
        for port in config.port_range.0..=config.port_range.1 {
            let addr: SocketAddr = format!("{}:{}", config.bind_address, port)
                .parse()
                .map_err(|e| anyhow!("Invalid bind address: {}", e))?;

            let listener = match TcpListener::bind(&addr).await {
                Ok(listener) => listener,
                Err(_) => continue, // Port already in use, try next
            };

            info!("SPICE proxy listening on {}", addr);

            let connections_clone = connections.clone();
            let config_clone = config.clone();

            tokio::spawn(async move {
                Self::handle_connections(listener, connections_clone, config_clone).await;
            });
        }

        // Wait for shutdown signal
        shutdown_rx.recv().await;
        Ok(())
    }

    /// Handle incoming WebSocket connections
    async fn handle_connections(
        listener: TcpListener,
        connections: Arc<RwLock<HashMap<String, SpiceConnection>>>,
        config: SpiceProxyConfig,
    ) {
        while let Ok((stream, addr)) = listener.accept().await {
            debug!("New WebSocket connection from {}", addr);

            let connections_clone = connections.clone();
            let config_clone = config.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_websocket_connection(
                    stream,
                    connections_clone,
                    config_clone,
                ).await {
                    error!("WebSocket connection error: {}", e);
                }
            });
        }
    }

    /// Handle individual WebSocket connection
    async fn handle_websocket_connection(
        stream: TcpStream,
        connections: Arc<RwLock<HashMap<String, SpiceConnection>>>,
        config: SpiceProxyConfig,
    ) -> Result<()> {
        let websocket = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = websocket.split();

        // Wait for authentication message
        let auth_message = timeout(config.auth_timeout, ws_receiver.next())
            .await
            .map_err(|_| anyhow!("Authentication timeout"))?
            .ok_or_else(|| anyhow!("Connection closed during auth"))?
            .map_err(|e| anyhow!("WebSocket error: {}", e))?;

        let auth_token = match auth_message {
            Message::Text(token) => token.to_string(),
            _ => return Err(anyhow!("Invalid authentication message")),
        };

        // Find connection by auth token
        let connection = {
            let connections_guard = connections.read().await;
            connections_guard
                .values()
                .find(|conn| conn.auth_token == auth_token)
                .cloned()
        };

        let connection = match connection {
            Some(conn) => conn,
            None => {
                let _ = ws_sender.send(Message::Text("Authentication failed".into())).await;
                return Err(anyhow!("Invalid authentication token"));
            }
        };

        info!(
            "Authenticated console connection for VM '{}' on SPICE port {}",
            connection.vm_id, connection.spice_port
        );

        // Update connection status
        {
            let mut connections_guard = connections.write().await;
            if let Some(conn) = connections_guard.get_mut(&format!("{}:{}", connection.vm_id, connection.websocket_port)) {
                conn.status = ConnectionStatus::Connected;
            }
        }

        // Send authentication success
        ws_sender.send(Message::Text("OK".into())).await?;

        // Connect to SPICE server
        let spice_stream = TcpStream::connect(format!("127.0.0.1:{}", connection.spice_port)).await?;
        let (mut spice_reader, mut spice_writer) = spice_stream.into_split();

        // Proxy data bidirectionally
        let ws_to_spice = async {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        if spice_writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => continue,
                }
            }
        };

        let spice_to_ws = async {
            let mut buffer = [0u8; 8192];
            while let Ok(n) = spice_reader.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                if ws_sender.send(Message::Binary(buffer[..n].to_vec().into())).await.is_err() {
                    break;
                }
            }
        };

        // Run both directions concurrently
        tokio::select! {
            _ = ws_to_spice => {},
            _ = spice_to_ws => {},
        }

        // Update connection status on disconnect
        {
            let mut connections_guard = connections.write().await;
            if let Some(conn) = connections_guard.get_mut(&format!("{}:{}", connection.vm_id, connection.websocket_port)) {
                conn.status = ConnectionStatus::Disconnected;
            }
        }

        info!("Console session ended for VM '{}'", connection.vm_id);
        Ok(())
    }

    /// Find an available port for WebSocket proxy
    async fn find_available_port(&self) -> Result<u16> {
        for port in self.config.port_range.0..=self.config.port_range.1 {
            let addr: SocketAddr = format!("{}:{}", self.config.bind_address, port).parse()?;
            
            if TcpListener::bind(&addr).await.is_ok() {
                return Ok(port);
            }
        }
        
        Err(anyhow!("No available ports in range {:?}", self.config.port_range))
    }

    /// Generate a cryptographically secure authentication token
    fn generate_auth_token() -> String {
        let mut rng = rand::thread_rng();
        let token: [u8; 32] = rng.gen();
        hex::encode(token)
    }

    /// Clean up expired connections
    pub async fn cleanup_expired_connections(&self) -> Result<()> {
        let now = SystemTime::now();
        let mut expired_connections = Vec::new();

        {
            let connections_guard = self.connections.read().await;
            for (id, connection) in connections_guard.iter() {
                if let Ok(elapsed) = now.duration_since(connection.created_at) {
                    if elapsed > self.config.connection_timeout {
                        expired_connections.push(id.clone());
                    }
                }
            }
        }

        if !expired_connections.is_empty() {
            let mut connections_guard = self.connections.write().await;
            for id in expired_connections {
                connections_guard.remove(&id);
                info!("Cleaned up expired connection: {}", id);
            }
        }

        Ok(())
    }
}

impl Drop for SpiceProxyService {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.try_send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spice_proxy_service_creation() {
        let config = SpiceProxyConfig::default();
        let service = SpiceProxyService::new(config);
        
        assert!(service.connections.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_auth_token_generation() {
        let token1 = SpiceProxyService::generate_auth_token();
        let token2 = SpiceProxyService::generate_auth_token();
        
        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 64); // 32 bytes * 2 (hex encoding)
    }

    #[tokio::test]
    async fn test_console_session_creation() {
        let config = SpiceProxyConfig::default();
        let service = SpiceProxyService::new(config);
        
        let result = service.create_console_session("test-vm".to_string(), 5930).await;
        assert!(result.is_ok());
        
        let console_info = result.unwrap();
        assert!(console_info.websocket_url.starts_with("ws://"));
        assert!(!console_info.auth_token.is_empty());
        assert!(!console_info.connection_id.is_empty());
    }
}