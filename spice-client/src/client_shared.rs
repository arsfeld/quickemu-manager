use crate::channels::main::MainChannel;
use crate::channels::display::DisplayChannel;
use crate::error::{Result, SpiceError};
use crate::protocol::ChannelType;
use crate::video::VideoOutput;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

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
    main_channel: Option<MainChannel>,
    display_channels: HashMap<u8, DisplayChannel>,
    #[cfg(not(target_arch = "wasm32"))]
    channel_tasks: Vec<JoinHandle<Result<()>>>,
    #[cfg(target_arch = "wasm32")]
    channel_tasks: Vec<TaskHandle>,
    video_output: Arc<VideoOutput>,
}

#[derive(Clone)]
pub struct SpiceClientShared {
    inner: Arc<Mutex<SpiceClientInner>>,
}

impl SpiceClientShared {
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
                channel_tasks: Vec::new(),
                video_output: Arc::new(VideoOutput::new()),
            })),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_websocket(websocket_url: String) -> Self {
        Self::new_websocket_with_auth(websocket_url, None)
    }

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
                channel_tasks: Vec::new(),
                video_output: Arc::new(VideoOutput::new()),
            })),
        }
    }
    
    pub async fn set_password(&mut self, password: String) {
        let mut inner = self.inner.lock().await;
        inner.password = Some(password);
    }

    pub async fn connect(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ref ws_url) = inner.websocket_url {
                info!("Connecting to SPICE server via WebSocket: {}", ws_url);
                let mut main_channel = MainChannel::new_websocket_with_password(ws_url, inner.auth_token.clone(), inner.password.clone()).await?;
                main_channel.initialize().await?;
                
                let channels = main_channel.get_channels_list().await?;
                info!("Available channels: {:?}", channels);
                info!("Skipping additional channels - using main channel only for now");
                
                inner.main_channel = Some(main_channel);
                return Ok(());
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            info!("Connecting to SPICE server at {}:{}", inner.host, inner.port);

            let mut main_channel = MainChannel::new(&inner.host, inner.port).await?;
            main_channel.initialize().await?;
            
            let channels = main_channel.get_channels_list().await?;
            info!("Available channels: {:?}", channels);

            for (channel_type, channel_id) in channels {
                match channel_type {
                    ChannelType::Display => {
                        let display_channel = DisplayChannel::new(&inner.host, inner.port, channel_id).await?;
                        inner.display_channels.insert(channel_id, display_channel);
                        info!("Connected to display channel {}", channel_id);
                    }
                    _ => {
                        info!("Ignoring channel type {:?} id {}", channel_type, channel_id);
                    }
                }
            }

            inner.main_channel = Some(main_channel);
            return Ok(());
        }

        Err(SpiceError::Protocol("No connection method available".to_string()))
    }

    pub async fn start_event_loop(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        
        if inner.main_channel.is_none() {
            return Err(SpiceError::Protocol("Not connected to main channel".to_string()));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(mut main_channel) = inner.main_channel.take() {
                let main_task = tokio::spawn(async move {
                    main_channel.run().await
                });
                inner.channel_tasks.push(main_task);
            }

            let mut display_channels = std::mem::take(&mut inner.display_channels);
            for (channel_id, mut display_channel) in display_channels {
                let display_task = tokio::spawn(async move {
                    display_channel.run().await
                });
                inner.channel_tasks.push(display_task);
                info!("Started event loop for display channel {}", channel_id);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(mut main_channel) = inner.main_channel.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = main_channel.run().await {
                        error!("Main channel error: {}", e);
                    }
                });
                inner.channel_tasks.push(());
            }

            let mut display_channels = std::mem::take(&mut inner.display_channels);
            for (channel_id, mut display_channel) in display_channels {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = display_channel.run().await {
                        error!("Display channel {} error: {}", channel_id, e);
                    }
                });
                inner.channel_tasks.push(());
                info!("Started event loop for display channel {}", channel_id);
            }
        }

        Ok(())
    }

    pub async fn get_display_surface(&self, channel_id: u8) -> Option<crate::channels::display::DisplaySurface> {
        let inner = self.inner.lock().await;
        inner.display_channels.get(&channel_id)?.get_primary_surface().cloned()
    }

    pub async fn get_video_output(&self) -> Arc<VideoOutput> {
        let inner = self.inner.lock().await;
        inner.video_output.clone()
    }

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
    }
}