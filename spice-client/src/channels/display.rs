use crate::channels::{Channel, ChannelConnection};
use crate::error::{Result, SpiceError};
use crate::protocol::*;
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[cfg(test)]
#[path = "display_tests.rs"]
mod display_tests;

#[path = "video_tests.rs"]
mod video_tests;

#[derive(Debug, Clone)]
pub struct DisplaySurface {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub id: u32,
    pub codec_type: u8,
    pub width: u32,
    pub height: u32,
    pub dest_rect: SpiceRect,
}

pub struct DisplayChannel {
    pub(crate) connection: ChannelConnection,
    surfaces: HashMap<u32, DisplaySurface>,
    monitors: Vec<SpiceHead>,
    active_streams: HashMap<u32, StreamInfo>,
}

impl DisplayChannel {
    pub async fn new(host: &str, port: u16, channel_id: u8) -> Result<Self> {
        let mut connection = ChannelConnection::new(host, port, ChannelType::Display, channel_id).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            surfaces: HashMap::new(),
            monitors: Vec::new(),
            active_streams: HashMap::new(),
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
            surfaces: HashMap::new(),
            monitors: Vec::new(),
            active_streams: HashMap::new(),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Display channel {} initialized", self.connection.channel_id);
        Ok(())
    }

    pub fn get_surface(&self, surface_id: u32) -> Option<&DisplaySurface> {
        self.surfaces.get(&surface_id)
    }

    pub fn get_primary_surface(&self) -> Option<&DisplaySurface> {
        self.surfaces.get(&0)
    }

    pub fn get_monitors(&self) -> &[SpiceHead] {
        &self.monitors
    }

    pub fn get_surfaces(&self) -> &HashMap<u32, DisplaySurface> {
        &self.surfaces
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
            
            // Create primary surface (ID 0)
            self.surfaces.insert(0, DisplaySurface {
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
                let draw_fill: SpiceDrawFill = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse DrawFill: {}", e)))?;
                
                let surface_id = draw_fill.base.surface_id;
                if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                    // Apply fill operation to surface
                    let rect = &draw_fill.base.box_rect;
                    let color = draw_fill.brush.color;
                    
                    // Simple fill implementation
                    for y in rect.top..rect.bottom {
                        for x in rect.left..rect.right {
                            if x >= 0 && y >= 0 && (x as u32) < surface.width && (y as u32) < surface.height {
                                let pixel_offset = ((y as u32 * surface.width + x as u32) * 4) as usize;
                                if pixel_offset + 3 < surface.data.len() {
                                    surface.data[pixel_offset] = (color & 0xFF) as u8;           // B
                                    surface.data[pixel_offset + 1] = ((color >> 8) & 0xFF) as u8;  // G
                                    surface.data[pixel_offset + 2] = ((color >> 16) & 0xFF) as u8; // R
                                    surface.data[pixel_offset + 3] = ((color >> 24) & 0xFF) as u8; // A
                                }
                            }
                        }
                    }
                    debug!("Applied fill to rect {:?} with color 0x{:08X}", rect, color);
                }
            }
            x if x == DisplayChannelMessage::DrawCopy as u16 => {
                debug!("Handle draw copy");
                let draw_copy: SpiceDrawCopy = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse DrawCopy: {}", e)))?;
                
                // TODO: Implement actual copy operation
                // This would involve copying pixels from src_image to the surface
                debug!("DrawCopy: dest rect {:?}, src rect {:?}", 
                       draw_copy.base.box_rect, draw_copy.src_area);
            }
            x if x == DisplayChannelMessage::DrawOpaque as u16 => {
                debug!("Handle draw opaque");
                let draw_opaque: SpiceDrawOpaque = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse DrawOpaque: {}", e)))?;
                
                // TODO: Implement actual opaque draw operation
                // This would involve drawing src_image with the specified brush
                debug!("DrawOpaque: dest rect {:?}, brush color 0x{:08X}", 
                       draw_opaque.base.box_rect, draw_opaque.brush.color);
            }
            x if x == DisplayChannelMessage::DrawBlend as u16 => {
                debug!("Handle draw blend");
                // TODO: Implement blend operation
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
                let stream_create: SpiceStreamCreate = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse StreamCreate: {}", e)))?;
                
                info!("Created stream {} - codec: {}, dimensions: {}x{} -> {}x{}", 
                      stream_create.id,
                      stream_create.codec_type,
                      stream_create.src_width,
                      stream_create.src_height,
                      stream_create.stream_width,
                      stream_create.stream_height);
                
                // Store stream info for processing subsequent data
                self.active_streams.insert(stream_create.id, StreamInfo {
                    id: stream_create.id,
                    codec_type: stream_create.codec_type,
                    width: stream_create.stream_width,
                    height: stream_create.stream_height,
                    dest_rect: stream_create.dest.clone(),
                });
            }
            x if x == DisplayChannelMessage::StreamData as u16 => {
                debug!("Handle stream data");
                let stream_data: SpiceStreamData = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse StreamData: {}", e)))?;
                
                debug!("Received {} bytes for stream {}", stream_data.data_size, stream_data.id);
                
                // TODO: Decode stream data and apply to surface
                // This would involve:
                // 1. Finding the decoder for this stream ID
                // 2. Decoding the data based on codec type
                // 3. Applying decoded frame to the display surface
            }
            x if x == DisplayChannelMessage::StreamDestroy as u16 => {
                debug!("Handle stream destroy");
                let stream_destroy: SpiceStreamDestroy = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse StreamDestroy: {}", e)))?;
                
                info!("Destroyed stream {}", stream_destroy.id);
                
                // Clean up stream info
                self.active_streams.remove(&stream_destroy.id);
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
                // Reset display state - clear all surfaces and streams
                self.surfaces.clear();
                self.active_streams.clear();
                self.monitors.clear();
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
            x if x == SPICE_MSG_DISPLAY_SURFACE_CREATE => {
                debug!("Received surface create");
                let surface_create: SpiceSurfaceCreate = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse SurfaceCreate: {}", e)))?;
                
                info!("Creating surface {} - {}x{} format: {}", 
                      surface_create.surface_id,
                      surface_create.width,
                      surface_create.height,
                      surface_create.format);
                
                // Create new surface
                let data_size = (surface_create.width * surface_create.height * 4) as usize;
                self.surfaces.insert(surface_create.surface_id, DisplaySurface {
                    width: surface_create.width,
                    height: surface_create.height,
                    format: surface_create.format,
                    data: vec![0; data_size],
                });
            }
            x if x == SPICE_MSG_DISPLAY_SURFACE_DESTROY => {
                debug!("Received surface destroy");
                let surface_destroy: SpiceSurfaceDestroy = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse SurfaceDestroy: {}", e)))?;
                
                info!("Destroying surface {}", surface_destroy.surface_id);
                
                self.surfaces.remove(&surface_destroy.surface_id);
            }
            x if x == SPICE_MSG_DISPLAY_MONITORS_CONFIG => {
                debug!("Received monitors config");
                let monitors_config: SpiceMonitorsConfig = bincode::deserialize(data)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse MonitorsConfig: {}", e)))?;
                
                info!("Monitors config: {} monitors (max {})", 
                      monitors_config.count, monitors_config.max_allowed);
                
                self.monitors = monitors_config.heads;
                
                // Log monitor configurations
                for (i, head) in self.monitors.iter().enumerate() {
                    info!("Monitor {}: {}x{} at ({},{}) on surface {}", 
                          i, head.width, head.height, head.x, head.y, head.surface_id);
                }
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