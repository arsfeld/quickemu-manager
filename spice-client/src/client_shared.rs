use crate::channels::main::MainChannel;
use crate::channels::display::DisplayChannel;
use crate::channels::inputs::InputsChannel;
use crate::channels::cursor::{CursorChannel, CursorShape};
use crate::channels::MouseButton;
use crate::error::{Result, SpiceError};
use crate::protocol::ChannelType;
use crate::video::{VideoOutput, create_video_output};
use crate::utils::sleep;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use instant::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle;

#[cfg(target_arch = "wasm32")]
type TaskHandle = ();

pub struct SpiceClientInner {
    host: String,
    port: u16,
    #[cfg(target_arch = "wasm32")]
    websocket_url: Option<String>,
    #[cfg(target_arch = "wasm32")]
    auth_token: Option<String>,
    password: Option<String>,
    main_channel: Option<Arc<Mutex<MainChannel>>>,
    display_channels: HashMap<u8, Arc<Mutex<DisplayChannel>>>,
    inputs_channels: HashMap<u8, Arc<Mutex<InputsChannel>>>,
    cursor_channels: HashMap<u8, Arc<Mutex<CursorChannel>>>,
    #[cfg(not(target_arch = "wasm32"))]
    channel_tasks: Vec<JoinHandle<Result<()>>>,
    #[cfg(target_arch = "wasm32")]
    channel_tasks: Vec<TaskHandle>,
    video_output: Arc<dyn VideoOutput>,
    #[cfg(target_arch = "wasm32")]
    error_state: Arc<std::sync::Mutex<Option<String>>>,
}

/// Shared SPICE client implementation that works on both native and WebAssembly targets.
///
/// This struct provides the core SPICE client functionality that is shared between
/// the native and WebAssembly implementations. It manages the connection to the SPICE
/// server, handles channel creation and communication, and provides methods for
/// interacting with the remote display.
///
/// The client is designed to be used asynchronously and is thread-safe through
/// internal locking. Multiple clones of the client can be created and used
/// concurrently.
///
/// # Example
///
/// ```no_run
/// use spice_client::SpiceClientShared;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut client = SpiceClientShared::new("localhost".to_string(), 5900);
/// client.set_password("my_password".to_string()).await;
/// client.connect().await?;
/// client.start_event_loop().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SpiceClientShared {
    inner: Arc<Mutex<SpiceClientInner>>,
}

impl SpiceClientShared {
    /// Creates a new SPICE client for connecting to the specified host and port.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname or IP address of the SPICE server
    /// * `port` - The port number of the SPICE server (typically 5900)
    ///
    /// # Example
    ///
    /// ```
    /// use spice_client::SpiceClientShared;
    ///
    /// let client = SpiceClientShared::new("localhost".to_string(), 5900);
    /// ```
    pub fn new(host: String, port: u16) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SpiceClientInner {
                host,
                port,
                #[cfg(target_arch = "wasm32")]
                websocket_url: None,
                #[cfg(target_arch = "wasm32")]
                auth_token: None,
                password: None,
                main_channel: None,
                display_channels: HashMap::new(),
                inputs_channels: HashMap::new(),
                cursor_channels: HashMap::new(),
                channel_tasks: Vec::new(),
                video_output: create_video_output(),
                #[cfg(target_arch = "wasm32")]
                error_state: Arc::new(std::sync::Mutex::new(None)),
            })),
        }
    }

    /// Creates a new SPICE client for WebAssembly using a WebSocket connection.
    ///
    /// This method is only available when compiling for WebAssembly targets.
    /// It creates a client that will connect through a WebSocket proxy instead
    /// of a direct TCP connection.
    ///
    /// # Arguments
    ///
    /// * `websocket_url` - The WebSocket URL (e.g., "ws://localhost:8080/spice")
    ///
    /// # Example
    ///
    /// ```ignore
    /// use spice_client::SpiceClientShared;
    ///
    /// let client = SpiceClientShared::new_websocket("ws://localhost:8080/spice".to_string());
    /// ```
    #[cfg(target_arch = "wasm32")]
    pub fn new_websocket(websocket_url: String) -> Self {
        Self::new_websocket_with_auth(websocket_url, None)
    }

    /// Creates a new SPICE client for WebAssembly with authentication token.
    ///
    /// This method is only available when compiling for WebAssembly targets.
    /// It creates a client that will connect through a WebSocket proxy with
    /// an optional authentication token for the proxy server.
    ///
    /// # Arguments
    ///
    /// * `websocket_url` - The WebSocket URL (e.g., "ws://localhost:8080/spice")
    /// * `auth_token` - Optional authentication token for the WebSocket proxy
    ///
    /// # Example
    ///
    /// ```ignore
    /// use spice_client::SpiceClientShared;
    ///
    /// let client = SpiceClientShared::new_websocket_with_auth(
    ///     "wss://proxy.example.com/spice".to_string(),
    ///     Some("bearer_token_123".to_string())
    /// );
    /// ```
    #[cfg(target_arch = "wasm32")]
    pub fn new_websocket_with_auth(websocket_url: String, auth_token: Option<String>) -> Self {
        let (host, port) = if websocket_url.contains("://") {
            let without_protocol = websocket_url.split("://").nth(1).unwrap_or("localhost:8080");
            let parts: Vec<&str> = without_protocol.split(':').collect();
            let host = parts.get(0).unwrap_or(&"localhost").to_string();
            let port = parts.get(1)
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080);
            (host, port)
        } else {
            ("websocket".to_string(), 0)
        };

        Self {
            inner: Arc::new(Mutex::new(SpiceClientInner {
                host,
                port,
                websocket_url: Some(websocket_url),
                auth_token,
                password: None,
                main_channel: None,
                display_channels: HashMap::new(),
                inputs_channels: HashMap::new(),
                cursor_channels: HashMap::new(),
                channel_tasks: Vec::new(),
                video_output: create_video_output(),
                #[cfg(target_arch = "wasm32")]
                error_state: Arc::new(std::sync::Mutex::new(None)),
            })),
        }
    }
    
    /// Sets the password for authenticating with the SPICE server.
    ///
    /// This password will be used during the connection handshake if the server
    /// requires authentication. The password should be set before calling `connect()`.
    ///
    /// # Arguments
    ///
    /// * `password` - The password or ticket for SPICE authentication
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use spice_client::SpiceClientShared;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = SpiceClientShared::new("localhost".to_string(), 5900);
    /// client.set_password("my_spice_password".to_string()).await;
    /// client.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_password(&mut self, password: String) {
        let mut inner = self.inner.lock().await;
        inner.password = Some(password);
    }
    
    /// Gets the current error state of the client (WebAssembly only).
    ///
    /// This method is only available on WebAssembly targets and returns any
    /// error message that occurred during connection or operation. This is useful
    /// for debugging connection issues in web browsers.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The error message if an error occurred
    /// * `None` - If no error has occurred
    #[cfg(target_arch = "wasm32")]
    pub async fn get_error_state(&self) -> Option<String> {
        let inner = self.inner.lock().await;
        let error_state = inner.error_state.clone();
        drop(inner);
        let result = error_state.lock().unwrap().clone();
        result
    }

    /// Connects to the SPICE server.
    ///
    /// This method establishes a connection to the SPICE server and performs
    /// the initial handshake. It will:
    ///
    /// 1. Connect to the main channel
    /// 2. Perform authentication if a password was set
    /// 3. Retrieve the list of available channels
    /// 4. Connect to display channels automatically
    ///
    /// # Errors
    ///
    /// Returns a `SpiceError` if:
    /// - The connection cannot be established
    /// - Authentication fails
    /// - The protocol handshake fails
    /// - The server version is incompatible
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use spice_client::SpiceClientShared;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = SpiceClientShared::new("localhost".to_string(), 5900);
    /// client.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ws_url) = inner.websocket_url.clone() {
                info!("Connecting to SPICE server via WebSocket: {}", ws_url);
                let mut main_channel = MainChannel::new_websocket_with_password(&ws_url, inner.auth_token.clone(), inner.password.clone()).await?;
                main_channel.initialize().await?;
                
                let channels = main_channel.get_channels_list().await?;
                info!("Available channels: {:?}", channels);
                
                // Get session_id from main channel (for information only)
                let session_id = main_channel.get_session_id();
                info!("Got session_id {:?} from main channel", session_id);
                
                // Connect to Display channel as it might be required
                for (channel_type, channel_id) in &channels {
                    match channel_type {
                        ChannelType::Display => {
                            info!("Connecting to display channel {} via WebSocket", channel_id);
                            // All channels in a new session use connection_id = 0
                            match DisplayChannel::new_websocket_with_auth_and_session(&ws_url, *channel_id, inner.auth_token.clone(), inner.password.clone(), Some(0)).await {
                                Ok(display_channel) => {
                                    inner.display_channels.insert(*channel_id, Arc::new(Mutex::new(display_channel)));
                                    info!("Connected to display channel {}", channel_id);
                                }
                                Err(e) => {
                                    warn!("Failed to connect to display channel {}: {}", channel_id, e);
                                }
                            }
                        }
                        ChannelType::Inputs => {
                            info!("Connecting to inputs channel {} via WebSocket", channel_id);
                            // All channels in a new session use connection_id = 0
                            match InputsChannel::new_websocket_with_auth_and_session(&ws_url, *channel_id, inner.auth_token.clone(), inner.password.clone(), Some(0)).await {
                                Ok(inputs_channel) => {
                                    inner.inputs_channels.insert(*channel_id, Arc::new(Mutex::new(inputs_channel)));
                                    info!("Connected to inputs channel {}", channel_id);
                                }
                                Err(e) => {
                                    warn!("Failed to connect to inputs channel {}: {}", channel_id, e);
                                }
                            }
                        }
                        ChannelType::Cursor => {
                            info!("Connecting to cursor channel {} via WebSocket", channel_id);
                            // All channels in a new session use connection_id = 0
                            match CursorChannel::new_websocket_with_auth_and_session(&ws_url, *channel_id, inner.auth_token.clone(), inner.password.clone(), Some(0)).await {
                                Ok(cursor_channel) => {
                                    inner.cursor_channels.insert(*channel_id, Arc::new(Mutex::new(cursor_channel)));
                                    info!("Connected to cursor channel {}", channel_id);
                                }
                                Err(e) => {
                                    warn!("Failed to connect to cursor channel {}: {}", channel_id, e);
                                }
                            }
                        }
                        _ => {
                            info!("Skipping channel type {:?} id {} for now", channel_type, channel_id);
                        }
                    }
                }
                
                inner.main_channel = Some(Arc::new(Mutex::new(main_channel)));
                return Ok(());
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            info!("Connecting to SPICE server at {}:{}", inner.host, inner.port);

            let mut main_channel = MainChannel::new(&inner.host, inner.port).await?;
            main_channel.initialize().await?;
            
            // Get the session_id from main channel
            let session_id = main_channel.get_session_id();
            info!("Got session_id from main channel: {:?}", session_id);
            
            // If session_id is None, the main channel didn't receive SPICE_MSG_MAIN_INIT yet
            if session_id.is_none() {
                error!("CRITICAL: Main channel initialization didn't provide session_id!");
                error!("The server should have sent SPICE_MSG_MAIN_INIT with session_id");
                return Err(SpiceError::Protocol("No session_id received from main channel".to_string()));
            }
            
            // Longer delay to ensure server has fully processed main channel initialization
            sleep(Duration::from_millis(500)).await;
            
            let channels = main_channel.get_channels_list().await?;
            info!("Available channels: {:?}", channels);

            // Try different connection_id approaches based on SPICE protocol understanding
            info!("Attempting to connect secondary channels");
            
            // Wait a bit more to ensure server is ready
            sleep(Duration::from_secs(1)).await;
            
            for (channel_type, channel_id) in channels {
                match channel_type {
                    ChannelType::Display => {
                        info!("Connecting to display channel {} with session_id as connection_id", channel_id);
                        
                        // According to SPICE protocol: non-main channels use session_id as connection_id
                        let display_channel = DisplayChannel::new_with_connection_id(&inner.host, inner.port, channel_id, session_id).await?;
                        inner.display_channels.insert(channel_id, Arc::new(Mutex::new(display_channel)));
                        info!("✓ Connected to display channel {} with connection_id = {}", channel_id, session_id.unwrap_or(0));
                    }
                    ChannelType::Inputs => {
                        info!("Connecting to inputs channel {} with session_id as connection_id", channel_id);
                        
                        // According to SPICE protocol: non-main channels use session_id as connection_id
                        let inputs_channel = InputsChannel::new_with_connection_id(&inner.host, inner.port, channel_id, session_id).await?;
                        inner.inputs_channels.insert(channel_id, Arc::new(Mutex::new(inputs_channel)));
                        info!("✓ Connected to inputs channel {} with connection_id = {}", channel_id, session_id.unwrap_or(0));
                    }
                    ChannelType::Cursor => {
                        info!("Connecting to cursor channel {} with session_id as connection_id", channel_id);
                        
                        // According to SPICE protocol: non-main channels use session_id as connection_id
                        let cursor_channel = CursorChannel::new_with_connection_id(&inner.host, inner.port, channel_id, session_id).await?;
                        inner.cursor_channels.insert(channel_id, Arc::new(Mutex::new(cursor_channel)));
                        info!("✓ Connected to cursor channel {} with connection_id = {}", channel_id, session_id.unwrap_or(0));
                    }
                    _ => {
                        info!("Ignoring channel type {:?} id {}", channel_type, channel_id);
                    }
                }
            }

            inner.main_channel = Some(Arc::new(Mutex::new(main_channel)));
            return Ok(());
        }

        Err(SpiceError::Protocol("No connection method available".to_string()))
    }

    /// Starts the event processing loops for all connected channels.
    ///
    /// This method must be called after `connect()` to begin processing incoming
    /// messages from the SPICE server. It spawns background tasks to handle
    /// messages for the main channel and all display channels.
    ///
    /// On native targets, this spawns Tokio tasks that run concurrently.
    /// On WebAssembly, this uses `spawn_local` to run tasks in the browser's
    /// event loop.
    ///
    /// # Errors
    ///
    /// Returns a `SpiceError` if the client is not connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use spice_client::SpiceClientShared;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = SpiceClientShared::new("localhost".to_string(), 5900);
    /// client.connect().await?;
    /// client.start_event_loop().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start_event_loop(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        
        if inner.main_channel.is_none() {
            return Err(SpiceError::Protocol("Not connected to main channel".to_string()));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(main_channel_arc) = inner.main_channel.clone() {
                let main_task = tokio::spawn(async move {
                    let mut main_channel = main_channel_arc.lock().await;
                    main_channel.run().await
                });
                inner.channel_tasks.push(main_task);
            }

            let display_channels: Vec<(u8, Arc<Mutex<DisplayChannel>>)> = inner.display_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, display_channel_arc) in display_channels {
                let display_task = tokio::spawn(async move {
                    let mut display_channel = display_channel_arc.lock().await;
                    display_channel.run().await
                });
                inner.channel_tasks.push(display_task);
                info!("Started event loop for display channel {}", channel_id);
            }

            let inputs_channels: Vec<(u8, Arc<Mutex<InputsChannel>>)> = inner.inputs_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, inputs_channel_arc) in inputs_channels {
                let inputs_task = tokio::spawn(async move {
                    let mut inputs_channel = inputs_channel_arc.lock().await;
                    inputs_channel.run().await
                });
                inner.channel_tasks.push(inputs_task);
                info!("Started event loop for inputs channel {}", channel_id);
            }
            
            let cursor_channels: Vec<(u8, Arc<Mutex<CursorChannel>>)> = inner.cursor_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, cursor_channel_arc) in cursor_channels {
                let cursor_task = tokio::spawn(async move {
                    let mut cursor_channel = cursor_channel_arc.lock().await;
                    cursor_channel.run().await
                });
                inner.channel_tasks.push(cursor_task);
                info!("Started event loop for cursor channel {}", channel_id);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let error_state = inner.error_state.clone();
            
            if let Some(main_channel_arc) = inner.main_channel.clone() {
                let error_state_clone = error_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut main_channel = main_channel_arc.lock().await;
                    if let Err(e) = main_channel.run().await {
                        error!("Main channel error: {}", e);
                        // Set error state to stop other operations
                        *error_state_clone.lock().unwrap() = Some(format!("Main channel error: {}", e));
                    }
                });
                inner.channel_tasks.push(());
            }

            let display_channels: Vec<(u8, Arc<Mutex<DisplayChannel>>)> = inner.display_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, display_channel_arc) in display_channels {
                let error_state_clone = error_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut display_channel = display_channel_arc.lock().await;
                    if let Err(e) = display_channel.run().await {
                        error!("Display channel {} error: {}", channel_id, e);
                        // Set error state to stop other operations
                        *error_state_clone.lock().unwrap() = Some(format!("Display channel {} error: {}", channel_id, e));
                    }
                });
                inner.channel_tasks.push(());
                info!("Started event loop for display channel {}", channel_id);
            }

            let inputs_channels: Vec<(u8, Arc<Mutex<InputsChannel>>)> = inner.inputs_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, inputs_channel_arc) in inputs_channels {
                let error_state_clone = error_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut inputs_channel = inputs_channel_arc.lock().await;
                    if let Err(e) = inputs_channel.run().await {
                        error!("Inputs channel {} error: {}", channel_id, e);
                        // Set error state to stop other operations
                        *error_state_clone.lock().unwrap() = Some(format!("Inputs channel {} error: {}", channel_id, e));
                    }
                });
                inner.channel_tasks.push(());
                info!("Started event loop for inputs channel {}", channel_id);
            }
            
            let cursor_channels: Vec<(u8, Arc<Mutex<CursorChannel>>)> = inner.cursor_channels.iter()
                .map(|(id, ch)| (*id, ch.clone()))
                .collect();
            for (channel_id, cursor_channel_arc) in cursor_channels {
                let error_state_clone = error_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut cursor_channel = cursor_channel_arc.lock().await;
                    if let Err(e) = cursor_channel.run().await {
                        error!("Cursor channel {} error: {}", channel_id, e);
                        // Set error state to stop other operations
                        *error_state_clone.lock().unwrap() = Some(format!("Cursor channel {} error: {}", channel_id, e));
                    }
                });
                inner.channel_tasks.push(());
                info!("Started event loop for cursor channel {}", channel_id);
            }
        }

        Ok(())
    }

    /// Gets the current cursor shape for a specific channel.
    ///
    /// Returns the current cursor shape data for the specified cursor channel,
    /// if available.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The ID of the cursor channel (usually 0)
    ///
    /// # Returns
    ///
    /// * `Some(CursorShape)` - The current cursor shape
    /// * `None` - If the channel doesn't exist or no cursor is set
    pub async fn get_cursor_shape(&self, channel_id: u8) -> Option<CursorShape> {
        let inner = self.inner.lock().await;
        if let Some(channel_arc) = inner.cursor_channels.get(&channel_id) {
            let channel = channel_arc.lock().await;
            channel.get_current_cursor().cloned()
        } else {
            None
        }
    }
    
    /// Gets the current display surface for a specific channel.
    ///
    /// Returns the primary display surface data for the specified display channel,
    /// if available. The surface contains the current screen dimensions, pixel format,
    /// and raw image data.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The ID of the display channel (usually 0 for the primary display)
    ///
    /// # Returns
    ///
    /// * `Some(DisplaySurface)` - The current display surface
    /// * `None` - If the channel doesn't exist or no surface is available yet
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use spice_client::SpiceClientShared;
    /// # async fn example(client: &SpiceClientShared) -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(surface) = client.get_display_surface(0).await {
    ///     println!("Display size: {}x{}", surface.width, surface.height);
    ///     println!("Pixel format: {:?}", surface.format);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_display_surface(&self, channel_id: u8) -> Option<crate::channels::display::DisplaySurface> {
        let inner = self.inner.lock().await;
        if let Some(channel_arc) = inner.display_channels.get(&channel_id) {
            let channel = channel_arc.lock().await;
            channel.get_primary_surface().cloned()
        } else {
            None
        }
    }

    /// Gets the video output handler for this client.
    ///
    /// Returns the video output implementation that processes and renders
    /// display updates. This can be used to integrate with custom rendering
    /// systems or to access video frames directly.
    ///
    /// # Returns
    ///
    /// An `Arc` to the video output implementation.
    pub async fn get_video_output(&self) -> Arc<dyn VideoOutput> {
        let inner = self.inner.lock().await;
        inner.video_output.clone()
    }

    /// Updates the video output with the latest display surface data.
    ///
    /// This method retrieves the current display surface from the specified
    /// channel and updates the video output handler with the new frame data.
    /// This is typically called automatically but can be used for manual updates.
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The ID of the display channel to update from
    ///
    /// # Errors
    ///
    /// Returns `Ok(())` even if no surface is available (no-op in that case).
    pub async fn update_video_from_display(&self, channel_id: u8) -> Result<()> {
        if let Some(surface) = self.get_display_surface(channel_id).await {
            let inner = self.inner.lock().await;
            inner.video_output.update_frame(&surface).await;
        }
        Ok(())
    }

    pub async fn wait_for_completion(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tasks = std::mem::take(&mut inner.channel_tasks);
            drop(inner); // Release lock before waiting
            
            for task in tasks {
                match task.await {
                    Ok(Ok(())) => {
                        info!("Channel task completed successfully");
                    }
                    Ok(Err(e)) => {
                        error!("Channel task failed: {}", e);
                        return Err(e);
                    }
                    Err(e) => {
                        error!("Task join error: {}", e);
                        return Err(SpiceError::Protocol(format!("Task join error: {}", e)));
                    }
                }
            }
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            inner.channel_tasks.clear();
            info!("WASM: Tasks are running in background, cannot wait for completion");
        }

        Ok(())
    }

    pub async fn disconnect(&self) {
        let mut inner = self.inner.lock().await;
        info!("Disconnecting from SPICE server");
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            for task in inner.channel_tasks.drain(..) {
                task.abort();
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            inner.channel_tasks.clear();
        }

        inner.main_channel = None;
        inner.display_channels.clear();
        inner.inputs_channels.clear();
    }

    // Input forwarding methods
    
    /// Sends a key down event to the specified inputs channel.
    pub async fn send_key_down(&self, channel_id: u8, scancode: u32) -> Result<()> {
        let inner = self.inner.lock().await;
        
        if let Some(inputs_channel_arc) = inner.inputs_channels.get(&channel_id) {
            let mut inputs_channel = inputs_channel_arc.lock().await;
            inputs_channel.send_key_down(scancode).await
        } else {
            Err(SpiceError::Protocol(format!("Inputs channel {} not connected", channel_id)))
        }
    }
    
    /// Sends a key up event to the specified inputs channel.
    pub async fn send_key_up(&self, channel_id: u8, scancode: u32) -> Result<()> {
        let inner = self.inner.lock().await;
        
        if let Some(inputs_channel_arc) = inner.inputs_channels.get(&channel_id) {
            let mut inputs_channel = inputs_channel_arc.lock().await;
            inputs_channel.send_key_up(scancode).await
        } else {
            Err(SpiceError::Protocol(format!("Inputs channel {} not connected", channel_id)))
        }
    }
    
    /// Sends a mouse motion event to the specified inputs channel.
    pub async fn send_mouse_motion(&self, channel_id: u8, x: i32, y: i32) -> Result<()> {
        let inner = self.inner.lock().await;
        
        if let Some(inputs_channel_arc) = inner.inputs_channels.get(&channel_id) {
            let mut inputs_channel = inputs_channel_arc.lock().await;
            inputs_channel.send_mouse_motion(x, y).await
        } else {
            Err(SpiceError::Protocol(format!("Inputs channel {} not connected", channel_id)))
        }
    }
    
    /// Sends a mouse button event to the specified inputs channel.
    pub async fn send_mouse_button(&self, channel_id: u8, button: MouseButton, pressed: bool) -> Result<()> {
        let inner = self.inner.lock().await;
        
        if let Some(inputs_channel_arc) = inner.inputs_channels.get(&channel_id) {
            let mut inputs_channel = inputs_channel_arc.lock().await;
            inputs_channel.send_mouse_button(button, pressed).await
        } else {
            Err(SpiceError::Protocol(format!("Inputs channel {} not connected", channel_id)))
        }
    }
    
    /// Sends a mouse wheel event to the specified inputs channel.
    pub async fn send_mouse_wheel(&self, channel_id: u8, delta_x: i32, delta_y: i32) -> Result<()> {
        let inner = self.inner.lock().await;
        
        if let Some(inputs_channel_arc) = inner.inputs_channels.get(&channel_id) {
            let mut inputs_channel = inputs_channel_arc.lock().await;
            // Convert wheel deltas to button presses (SPICE protocol uses button events for wheel)
            if delta_y > 0 {
                inputs_channel.send_mouse_button(MouseButton::WheelUp, true).await?;
                inputs_channel.send_mouse_button(MouseButton::WheelUp, false).await
            } else if delta_y < 0 {
                inputs_channel.send_mouse_button(MouseButton::WheelDown, true).await?;
                inputs_channel.send_mouse_button(MouseButton::WheelDown, false).await
            } else {
                Ok(())
            }
        } else {
            Err(SpiceError::Protocol(format!("Inputs channel {} not connected", channel_id)))
        }
    }
    
    /// Sets a callback that will be called whenever a display surface is updated.
    /// This allows external code to be notified when new frames are ready.
    pub async fn set_display_update_callback<F>(&self, channel_id: u8, callback: F) -> Result<()>
    where
        F: Fn(&crate::channels::display::DisplaySurface) + Send + Sync + 'static,
    {
        let inner = self.inner.lock().await;
        
        if let Some(display_channel_arc) = inner.display_channels.get(&channel_id) {
            let mut display_channel = display_channel_arc.lock().await;
            display_channel.set_update_callback(callback);
            Ok(())
        } else {
            Err(SpiceError::Protocol(format!("Display channel {} not connected", channel_id)))
        }
    }
}