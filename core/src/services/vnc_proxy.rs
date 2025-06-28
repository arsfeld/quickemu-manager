use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use rand::{thread_rng, Rng};
use hex;

#[derive(Debug, Clone)]
pub struct VncConnection {
    pub id: String,
    pub vm_id: String,
    pub vnc_host: String,
    pub vnc_port: u16,
    pub websocket_port: u16,
    pub auth_token: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ConsoleProtocol {
    Vnc,
    Spice,
}

#[derive(Debug, Clone)]
pub struct ConsoleInfo {
    pub websocket_url: String,
    pub auth_token: String,
    pub connection_id: String,
    pub protocol: ConsoleProtocol,
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Authenticating,
    Connected,
    Disconnected,
    Error(String),
}

pub struct VncProxy {
    connections: Arc<RwLock<HashMap<String, VncConnection>>>,
    active_proxies: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl VncProxy {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            active_proxies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_connection(
        &self,
        vm_id: String,
        vnc_host: String,
        vnc_port: u16,
    ) -> Result<VncConnection> {
        log::info!("Creating VNC connection for VM '{}' at {}:{}", vm_id, vnc_host, vnc_port);
        
        let connection_id = self.generate_connection_id();
        let auth_token = self.generate_auth_token();
        
        log::debug!("Generated connection ID: {}", connection_id);
        
        // Find an available port for WebSocket
        let websocket_port = self.find_available_port().await?;
        log::info!("Found available WebSocket port: {}", websocket_port);
        
        let connection = VncConnection {
            id: connection_id.clone(),
            vm_id: vm_id.clone(),
            vnc_host: vnc_host.clone(),
            vnc_port,
            websocket_port,
            auth_token: auth_token.clone(),
            status: "connecting".to_string(),
        };
        
        // Store the connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), connection.clone());
            log::debug!("Stored connection in map, total connections: {}", connections.len());
        }
        
        // Start the WebSocket proxy
        let connections_clone = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        let vnc_addr = format!("{}:{}", vnc_host, vnc_port);
        
        log::info!("Starting WebSocket proxy for connection {} on port {}", connection_id, websocket_port);
        
        let proxy_task = tokio::spawn(async move {
            log::debug!("WebSocket proxy task started for connection {}", connection_id_clone);
            if let Err(e) = Self::run_websocket_proxy(
                websocket_port,
                vnc_addr,
                auth_token,
                connections_clone,
                connection_id_clone,
            ).await {
                log::error!("WebSocket proxy error: {}", e);
            }
            log::debug!("WebSocket proxy task ended");
        });
        
        // Store the proxy task
        {
            let mut proxies = self.active_proxies.lock().await;
            proxies.insert(connection_id.clone(), proxy_task);
            log::debug!("Stored proxy task, active proxies: {}", proxies.len());
        }
        
        Ok(connection)
    }

    async fn run_websocket_proxy(
        websocket_port: u16,
        vnc_addr: String,
        expected_token: String,
        connections: Arc<RwLock<HashMap<String, VncConnection>>>,
        connection_id: String,
    ) -> Result<()> {
        let ws_addr = format!("0.0.0.0:{}", websocket_port);
        log::debug!("Attempting to bind WebSocket listener on {}", ws_addr);
        
        let ws_listener = TcpListener::bind(&ws_addr).await?;
        
        log::info!("VNC WebSocket proxy listening on {} for connection {}", ws_addr, connection_id);
        
        // Update connection status
        {
            let mut conns = connections.write().await;
            if let Some(conn) = conns.get_mut(&connection_id) {
                conn.status = "ready".to_string();
                log::debug!("Updated connection {} status to 'ready'", connection_id);
            }
        }
        
        log::debug!("Waiting for WebSocket connections on port {}", websocket_port);
        
        while let Ok((stream, addr)) = ws_listener.accept().await {
            log::info!("Accepted WebSocket connection from {} for connection {}", addr, connection_id);
            
            let vnc_addr = vnc_addr.clone();
            let expected_token = expected_token.clone();
            let connections = connections.clone();
            let connection_id = connection_id.clone();
            
            tokio::spawn(async move {
                log::debug!("Spawned handler for WebSocket connection from {}", addr);
                if let Err(e) = Self::handle_websocket_connection(
                    stream,
                    vnc_addr,
                    expected_token,
                    connections,
                    connection_id,
                ).await {
                    log::error!("WebSocket connection error: {}", e);
                }
            });
        }
        
        log::warn!("WebSocket proxy listener ended for connection {}", connection_id);
        Ok(())
    }

    async fn handle_websocket_connection(
        stream: TcpStream,
        vnc_addr: String,
        expected_token: String,
        connections: Arc<RwLock<HashMap<String, VncConnection>>>,
        connection_id: String,
    ) -> Result<()> {
        log::debug!("Handling WebSocket connection for connection {}", connection_id);
        
        let ws_stream = accept_async(stream).await?;
        log::debug!("WebSocket handshake completed for connection {}", connection_id);
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // First message should be authentication
        log::debug!("Waiting for authentication token from client");
        if let Some(Ok(Message::Text(auth_msg))) = ws_receiver.next().await {
            log::debug!("Received auth message, validating token");
            if auth_msg != expected_token {
                log::warn!("Invalid authentication token received for connection {}", connection_id);
                ws_sender.send(Message::Text("Authentication failed".to_string().into())).await?;
                return Err(anyhow!("Invalid authentication token"));
            }
            log::info!("Authentication successful for connection {}", connection_id);
        } else {
            log::error!("No authentication message received for connection {}", connection_id);
            return Err(anyhow!("No authentication message received"));
        }
        
        // Send authentication success
        ws_sender.send(Message::Text("authenticated".to_string().into())).await?;
        log::debug!("Sent authentication success message");
        
        // Update connection status
        {
            let mut conns = connections.write().await;
            if let Some(conn) = conns.get_mut(&connection_id) {
                conn.status = "connected".to_string();
                log::info!("Updated connection {} status to 'connected'", connection_id);
            }
        }
        
        // Connect to VNC/SPICE server
        // Note: This proxy currently only works with VNC protocol
        log::info!("Attempting to connect to console server at {}", vnc_addr);
        match TcpStream::connect(&vnc_addr).await {
            Ok(mut vnc_stream) => {
                log::info!("Successfully connected to console server at {}", vnc_addr);
                // TODO: Detect protocol and handle accordingly
                // Currently only VNC protocol is supported
                let (vnc_reader, vnc_writer) = vnc_stream.split();
                
                // Create bidirectional proxy
                let ws_to_vnc = Self::proxy_ws_to_vnc(ws_receiver, vnc_writer);
                let vnc_to_ws = Self::proxy_vnc_to_ws(vnc_reader, ws_sender);
                
                log::debug!("Starting bidirectional proxy for connection {}", connection_id);
                
                // Run both directions concurrently
                tokio::select! {
                    result = ws_to_vnc => {
                        if let Err(e) = result {
                            log::error!("WS to VNC proxy error for connection {}: {}", connection_id, e);
                        } else {
                            log::debug!("WS to VNC proxy ended normally for connection {}", connection_id);
                        }
                    }
                    result = vnc_to_ws => {
                        if let Err(e) = result {
                            log::error!("VNC to WS proxy error for connection {}: {}", connection_id, e);
                        } else {
                            log::debug!("VNC to WS proxy ended normally for connection {}", connection_id);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to connect to VNC server at {}: {}", vnc_addr, e);
                return Err(anyhow!("Failed to connect to VNC server: {}", e));
            }
        }
        
        // Update connection status
        {
            let mut conns = connections.write().await;
            if let Some(conn) = conns.get_mut(&connection_id) {
                conn.status = "disconnected".to_string();
                log::info!("Updated connection {} status to 'disconnected'", connection_id);
            }
        }
        
        log::debug!("WebSocket connection handler completed for connection {}", connection_id);
        Ok(())
    }

    async fn proxy_ws_to_vnc(
        mut ws_receiver: futures_util::stream::SplitStream<WebSocketStream<TcpStream>>,
        mut vnc_writer: tokio::net::tcp::WriteHalf<'_>,
    ) -> Result<()> {
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Binary(data) => {
                    vnc_writer.write_all(&data).await?;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }

    async fn proxy_vnc_to_ws(
        mut vnc_reader: tokio::net::tcp::ReadHalf<'_>,
        mut ws_sender: futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 65536];
        loop {
            let n = vnc_reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            ws_sender.send(Message::Binary(buffer[..n].to_vec().into())).await?;
        }
        Ok(())
    }

    pub async fn stop_connection(&self, connection_id: &str) -> Result<()> {
        // Remove the connection
        {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id);
        }
        
        // Cancel the proxy task
        {
            let mut proxies = self.active_proxies.lock().await;
            if let Some(task) = proxies.remove(connection_id) {
                task.abort();
            }
        }
        
        Ok(())
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<VncConnection> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    pub async fn get_connection_status(&self, connection_id: &str) -> Option<String> {
        let connections = self.connections.read().await;
        connections.get(connection_id).map(|c| c.status.clone())
    }

    async fn find_available_port(&self) -> Result<u16> {
        // Try to find an available port in the range 6080-6099
        for port in 6080..6100 {
            if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{}", port)).await {
                drop(listener);
                return Ok(port);
            }
        }
        Err(anyhow!("No available ports found"))
    }

    fn generate_connection_id(&self) -> String {
        let mut rng = thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }

    fn generate_auth_token(&self) -> String {
        let mut rng = thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }
}

impl Default for VncProxy {
    fn default() -> Self {
        Self::new()
    }
}