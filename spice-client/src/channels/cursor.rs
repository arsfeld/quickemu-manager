//! Cursor channel implementation for hardware cursor support

use crate::channels::{Channel, ChannelConnection};
use crate::error::Result;
use crate::protocol::*;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Cursor shape data
#[derive(Debug, Clone)]
pub struct CursorShape {
    pub width: u16,
    pub height: u16,
    pub hot_spot_x: u16,
    pub hot_spot_y: u16,
    pub data: Vec<u8>,
    pub mask: Option<Vec<u8>>,
}

/// Cursor channel for handling mouse cursor updates
pub struct CursorChannel {
    pub(crate) connection: ChannelConnection,
    current_cursor: Option<CursorShape>,
    cursor_cache: HashMap<u64, CursorShape>,
    cursor_visible: bool,
    cursor_position: (i32, i32),
}

impl CursorChannel {
    pub async fn new(host: &str, port: u16, channel_id: u8) -> Result<Self> {
        let mut connection = ChannelConnection::new(host, port, ChannelType::Cursor, channel_id).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            current_cursor: None,
            cursor_cache: HashMap::new(),
            cursor_visible: true,
            cursor_position: (0, 0),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(websocket_url: &str, channel_id: u8) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_id, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(websocket_url: &str, channel_id: u8, auth_token: Option<String>) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(websocket_url, ChannelType::Cursor, channel_id, auth_token).await?;
        connection.handshake().await?;
        
        Ok(Self {
            connection,
            current_cursor: None,
            cursor_cache: HashMap::new(),
            cursor_visible: true,
            cursor_position: (0, 0),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Cursor channel {} initialized", self.connection.channel_id);
        Ok(())
    }

    pub fn get_current_cursor(&self) -> Option<&CursorShape> {
        self.current_cursor.as_ref()
    }

    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn get_cursor_position(&self) -> (i32, i32) {
        self.cursor_position
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let (header, data) = self.connection.read_message().await?;
            self.handle_message(&header, &data).await?;
        }
    }

    async fn handle_cursor_init(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 8 {
            let visible = u16::from_le_bytes([data[0], data[1]]) != 0;
            let x = i16::from_le_bytes([data[2], data[3]]) as i32;
            let y = i16::from_le_bytes([data[4], data[5]]) as i32;
            let trail_len = u16::from_le_bytes([data[6], data[7]]);
            
            self.cursor_visible = visible;
            self.cursor_position = (x, y);
            
            info!("Cursor init - visible: {}, position: ({}, {}), trail: {}", 
                  visible, x, y, trail_len);
        }
        Ok(())
    }

    async fn handle_cursor_set(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 17 {
            let cursor_header = SpiceCursorHeader {
                unique: u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
                type_: data[8],
                width: u16::from_le_bytes([data[9], data[10]]),
                height: u16::from_le_bytes([data[11], data[12]]),
                hot_spot_x: u16::from_le_bytes([data[13], data[14]]),
                hot_spot_y: u16::from_le_bytes([data[15], data[16]]),
            };
            
            let data_offset = 17;
            let data_size = (cursor_header.width * cursor_header.height * 4) as usize;
            
            if data.len() >= data_offset + data_size {
                let cursor_data = data[data_offset..data_offset + data_size].to_vec();
                
                let cursor_shape = CursorShape {
                    width: cursor_header.width,
                    height: cursor_header.height,
                    hot_spot_x: cursor_header.hot_spot_x,
                    hot_spot_y: cursor_header.hot_spot_y,
                    data: cursor_data,
                    mask: None,
                };
                
                // Cache the cursor
                self.cursor_cache.insert(cursor_header.unique, cursor_shape.clone());
                self.current_cursor = Some(cursor_shape);
                
                info!("Set cursor - {}x{}, hotspot: ({}, {})", 
                      cursor_header.width, cursor_header.height,
                      cursor_header.hot_spot_x, cursor_header.hot_spot_y);
            }
        }
        Ok(())
    }

    async fn handle_cursor_move(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 4 {
            let x = i16::from_le_bytes([data[0], data[1]]) as i32;
            let y = i16::from_le_bytes([data[2], data[3]]) as i32;
            
            self.cursor_position = (x, y);
            debug!("Cursor moved to ({}, {})", x, y);
        }
        Ok(())
    }

    async fn handle_cursor_hide(&mut self) -> Result<()> {
        self.cursor_visible = false;
        debug!("Cursor hidden");
        Ok(())
    }

    async fn handle_cursor_trail(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 4 {
            let length = u16::from_le_bytes([data[0], data[1]]);
            let frequency = u16::from_le_bytes([data[2], data[3]]);
            
            debug!("Cursor trail - length: {}, frequency: {}", length, frequency);
        }
        Ok(())
    }

    async fn handle_cursor_inval_one(&mut self, data: &[u8]) -> Result<()> {
        if data.len() >= 8 {
            let cache_id = u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
            
            self.cursor_cache.remove(&cache_id);
            debug!("Invalidated cursor cache entry: {}", cache_id);
        }
        Ok(())
    }

    async fn handle_cursor_inval_all(&mut self) -> Result<()> {
        self.cursor_cache.clear();
        debug!("Invalidated all cursor cache entries");
        Ok(())
    }
}

impl Channel for CursorChannel {
    async fn handle_message(&mut self, header: &SpiceDataHeader, data: &[u8]) -> Result<()> {
        match header.msg_type {
            SPICE_MSG_CURSOR_INIT => {
                debug!("Received cursor init");
                self.handle_cursor_init(data).await?;
            }
            SPICE_MSG_CURSOR_SET => {
                debug!("Received cursor set");
                self.handle_cursor_set(data).await?;
            }
            SPICE_MSG_CURSOR_MOVE => {
                debug!("Received cursor move");
                self.handle_cursor_move(data).await?;
            }
            SPICE_MSG_CURSOR_HIDE => {
                debug!("Received cursor hide");
                self.handle_cursor_hide().await?;
            }
            SPICE_MSG_CURSOR_TRAIL => {
                debug!("Received cursor trail");
                self.handle_cursor_trail(data).await?;
            }
            SPICE_MSG_CURSOR_INVAL_ONE => {
                debug!("Received cursor inval one");
                self.handle_cursor_inval_one(data).await?;
            }
            SPICE_MSG_CURSOR_INVAL_ALL => {
                debug!("Received cursor inval all");
                self.handle_cursor_inval_all().await?;
            }
            _ => {
                warn!("Unknown cursor message type: {}", header.msg_type);
            }
        }
        
        Ok(())
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Cursor
    }
}


#[derive(Debug, Clone)]
pub struct SpiceCursorHeader {
    pub unique: u64,
    pub type_: u8,
    pub width: u16,
    pub height: u16,
    pub hot_spot_x: u16,
    pub hot_spot_y: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cursor_shape_creation() {
        let cursor = CursorShape {
            width: 32,
            height: 32,
            hot_spot_x: 16,
            hot_spot_y: 16,
            data: vec![0xFF; 32 * 32 * 4],
            mask: None,
        };

        assert_eq!(cursor.width, 32);
        assert_eq!(cursor.height, 32);
        assert_eq!(cursor.hot_spot_x, 16);
        assert_eq!(cursor.hot_spot_y, 16);
        assert_eq!(cursor.data.len(), 32 * 32 * 4);
    }

    #[tokio::test]
    async fn test_cursor_position() {
        // Test cursor position parsing
        let move_data = vec![
            0x10, 0x00, // x = 16
            0x20, 0x00, // y = 32
        ];

        let x = i16::from_le_bytes([move_data[0], move_data[1]]) as i32;
        let y = i16::from_le_bytes([move_data[2], move_data[3]]) as i32;

        assert_eq!(x, 16);
        assert_eq!(y, 32);
    }
}