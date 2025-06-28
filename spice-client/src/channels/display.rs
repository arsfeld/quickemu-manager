use crate::channels::{Channel, ChannelConnection};
use crate::error::{Result, SpiceError};
use crate::protocol::*;
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct DisplaySurface {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub data: Vec<u8>,
}

pub struct DisplayChannel {
    connection: ChannelConnection,
    primary_surface: Option<DisplaySurface>,
}

impl DisplayChannel {
    pub async fn new(host: &str, port: u16, channel_id: u8) -> Result<Self> {
        let mut connection = ChannelConnection::new(host, port, ChannelType::Display, channel_id).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            primary_surface: None,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(websocket_url: &str, channel_id: u8) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_id, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(websocket_url: &str, channel_id: u8, auth_token: Option<String>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Display, channel_id, auth_token).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            primary_surface: None,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Display channel {} initialized", self.connection.channel_id);
        Ok(())
    }

    pub fn get_primary_surface(&self) -> Option<&DisplaySurface> {
        self.primary_surface.as_ref()
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let (header, data) = self.connection.read_message().await?;
            self.handle_message(&header, &data).await?;
        }
    }

    async fn handle_mode_message(&mut self, data: &[u8]) -> Result<()> {
        // Parse mode message to get display dimensions and format
        // This is a simplified implementation
        if data.len() >= 12 {
            let width = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let height = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            let format = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
            
            info!("Display mode: {}x{}, format: {}", width, height, format);
            
            // Create primary surface
            self.primary_surface = Some(DisplaySurface {
                width,
                height,
                format,
                data: vec![0; (width * height * 4) as usize], // Assuming 32-bit RGBA
            });
        }
        
        Ok(())
    }

    async fn handle_draw_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        match msg_type {
            x if x == DisplayChannelMessage::DrawFill as u16 => {
                debug!("Handle draw fill");
                // Parse and handle fill operation
            }
            x if x == DisplayChannelMessage::DrawCopy as u16 => {
                debug!("Handle draw copy");
                // Parse and handle copy operation
            }
            x if x == DisplayChannelMessage::DrawOpaque as u16 => {
                debug!("Handle draw opaque");
                // Parse and handle opaque draw
            }
            _ => {
                debug!("Unhandled draw message type: {}", msg_type);
            }
        }
        
        Ok(())
    }

    async fn handle_stream_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        match msg_type {
            x if x == DisplayChannelMessage::StreamCreate as u16 => {
                debug!("Handle stream create");
                // Parse and handle stream creation
            }
            x if x == DisplayChannelMessage::StreamData as u16 => {
                debug!("Handle stream data");
                // Parse and handle stream data
            }
            x if x == DisplayChannelMessage::StreamDestroy as u16 => {
                debug!("Handle stream destroy");
                // Parse and handle stream destruction
            }
            _ => {
                debug!("Unhandled stream message type: {}", msg_type);
            }
        }
        
        Ok(())
    }
}

impl Channel for DisplayChannel {
    async fn handle_message(&mut self, header: &SpiceDataHeader, data: &[u8]) -> Result<()> {
        match header.msg_type {
            x if x == DisplayChannelMessage::Mode as u16 => {
                debug!("Received display mode");
                self.handle_mode_message(data).await?;
            }
            x if x == DisplayChannelMessage::Mark as u16 => {
                debug!("Received display mark");
                // Handle mark message
            }
            x if x == DisplayChannelMessage::Reset as u16 => {
                debug!("Received display reset");
                // Reset display state
                self.primary_surface = None;
            }
            x if x == DisplayChannelMessage::InvalList as u16 => {
                debug!("Received invalidation list");
                // Handle invalidation list
            }
            x if x == DisplayChannelMessage::InvalAllPixmaps as u16 => {
                debug!("Received invalidate all pixmaps");
                // Handle pixmap invalidation
            }
            x if (DisplayChannelMessage::DrawFill as u16..=DisplayChannelMessage::DrawAlphaBlend as u16).contains(&x) => {
                self.handle_draw_message(header.msg_type, data).await?;
            }
            x if (DisplayChannelMessage::StreamCreate as u16..=DisplayChannelMessage::StreamDestroyAll as u16).contains(&x) => {
                self.handle_stream_message(header.msg_type, data).await?;
            }
            _ => {
                warn!("Unknown display message type: {}", header.msg_type);
            }
        }
        
        Ok(())
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Display
    }
}