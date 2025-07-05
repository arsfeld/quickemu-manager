use crate::channels::{Channel, ChannelConnection};
use crate::error::{Result, SpiceError};
use crate::protocol::*;
use binrw::BinRead;
use std::collections::HashMap;
use tracing::{debug, info, warn};

// Integration tests moved to tests/display_integration.rs

#[path = "video_tests.rs"]
mod video_tests;

// Cache for images referenced by FROM_CACHE types
struct ImageCache {
    entries: HashMap<u64, CachedImage>,
}

struct CachedImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn get(&self, id: u64) -> Option<&CachedImage> {
        self.entries.get(&id)
    }

    fn insert(&mut self, id: u64, data: Vec<u8>, width: u32, height: u32) {
        self.entries.insert(
            id,
            CachedImage {
                data,
                width,
                height,
            },
        );
    }
}

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
    update_callback: Option<Box<dyn Fn(&DisplaySurface) + Send + Sync>>,
    image_cache: ImageCache,
}

impl DisplayChannel {
    pub async fn new(host: &str, port: u16, channel_id: u8) -> Result<Self> {
        Self::new_with_connection_id(host, port, channel_id, None).await
    }

    pub async fn new_with_connection_id(
        host: &str,
        port: u16,
        channel_id: u8,
        connection_id: Option<u32>,
    ) -> Result<Self> {
        let mut connection =
            ChannelConnection::new(host, port, ChannelType::Display, channel_id).await?;
        if let Some(conn_id) = connection_id {
            connection.set_connection_id(conn_id);
        }
        connection.handshake().await?;

        // Send display init message after handshake
        info!("Sending SPICE_MSGC_DISPLAY_INIT");

        // Create the display init message
        let display_init = SpiceMsgcDisplayInit {
            cache_id: 0,
            cache_size: 0, // No cache for now
            glz_dict_id: 0,
        };

        // Serialize the message
        use binrw::BinWrite;
        let mut cursor = std::io::Cursor::new(Vec::new());
        display_init
            .write(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write display init: {e}")))?;
        let init_data = cursor.into_inner();

        connection
            .send_message(SPICE_MSGC_DISPLAY_INIT, &init_data)
            .await?;

        Ok(Self {
            connection,
            surfaces: HashMap::new(),
            monitors: Vec::new(),
            active_streams: HashMap::new(),
            update_callback: None,
            image_cache: ImageCache::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket(websocket_url: &str, channel_id: u8) -> Result<Self> {
        Self::new_websocket_with_auth(websocket_url, channel_id, None).await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth(
        websocket_url: &str,
        channel_id: u8,
        auth_token: Option<String>,
    ) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(
            websocket_url,
            ChannelType::Display,
            channel_id,
            auth_token,
        )
        .await?;
        connection.handshake().await?;

        // Send display init message after handshake
        info!("Sending SPICE_MSGC_DISPLAY_INIT");

        // Create the display init message
        let display_init = SpiceMsgcDisplayInit {
            cache_id: 0,
            cache_size: 0, // No cache for now
            glz_dict_id: 0,
        };

        // Serialize the message
        use binrw::BinWrite;
        let mut cursor = std::io::Cursor::new(Vec::new());
        display_init
            .write(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write display init: {e}")))?;
        let init_data = cursor.into_inner();

        connection
            .send_message(SPICE_MSGC_DISPLAY_INIT, &init_data)
            .await?;

        Ok(Self {
            connection,
            surfaces: HashMap::new(),
            monitors: Vec::new(),
            active_streams: HashMap::new(),
            update_callback: None,
            image_cache: ImageCache::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new_websocket_with_auth_and_session(
        websocket_url: &str,
        channel_id: u8,
        auth_token: Option<String>,
        password: Option<String>,
        connection_id: Option<u32>,
    ) -> Result<Self> {
        let mut connection = ChannelConnection::new_websocket_with_auth(
            websocket_url,
            ChannelType::Display,
            channel_id,
            auth_token,
        )
        .await?;
        if let Some(pwd) = password {
            connection.set_password(pwd);
        }
        if let Some(conn_id) = connection_id {
            connection.set_connection_id(conn_id);
        }
        connection.handshake().await?;

        // Send display init message after handshake
        info!("Sending SPICE_MSGC_DISPLAY_INIT");

        // Create the display init message
        let display_init = SpiceMsgcDisplayInit {
            cache_id: 0,
            cache_size: 0, // No cache for now
            glz_dict_id: 0,
        };

        // Serialize the message
        use binrw::BinWrite;
        let mut cursor = std::io::Cursor::new(Vec::new());
        display_init
            .write(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to write display init: {e}")))?;
        let init_data = cursor.into_inner();

        connection
            .send_message(SPICE_MSGC_DISPLAY_INIT, &init_data)
            .await?;

        Ok(Self {
            connection,
            surfaces: HashMap::new(),
            monitors: Vec::new(),
            active_streams: HashMap::new(),
            update_callback: None,
            image_cache: ImageCache::new(),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Display channel {} initialized", self.connection.channel_id);

        // According to SPICE protocol, display channels might need to send an init message
        // or wait for the server to send display configuration
        // For now, we just wait for server messages

        Ok(())
    }

    pub fn get_surface(&self, surface_id: u32) -> Option<&DisplaySurface> {
        self.surfaces.get(&surface_id)
    }

    pub fn get_primary_surface(&self) -> Option<&DisplaySurface> {
        let surface = self.surfaces.get(&0);
        if surface.is_none() {
            eprintln!(
                "DisplayChannel: No primary surface available. Total surfaces: {}",
                self.surfaces.len()
            );
        }
        surface
    }

    pub fn get_monitors(&self) -> &[SpiceHead] {
        &self.monitors
    }

    pub fn get_surfaces(&self) -> &HashMap<u32, DisplaySurface> {
        &self.surfaces
    }

    pub fn set_update_callback<F>(&mut self, callback: F)
    where
        F: Fn(&DisplaySurface) + Send + Sync + 'static,
    {
        self.update_callback = Some(Box::new(callback));
    }

    /// Process a single message from the server
    /// This is primarily for testing purposes
    pub async fn process_next_message(&mut self) -> Result<()> {
        let (header, data) = self.connection.read_message().await?;
        self.handle_message(&header, &data).await
    }

    fn notify_update(&self, surface_id: u32) {
        if let Some(ref callback) = self.update_callback {
            if let Some(surface) = self.surfaces.get(&surface_id) {
                callback(surface);
            }
        }
    }

    /// Resolve a SpiceAddress to get data from the message buffer
    /// SpiceAddress is an offset from the beginning of the message data
    fn resolve_address<'a>(&self, address: SpiceAddress, data: &'a [u8]) -> Option<&'a [u8]> {
        if address == 0 {
            return None;
        }

        // Check if this is an encoded address (upper 32 bits non-zero)
        if address > 0xFFFFFFFF {
            let surface_id = (address >> 32) as u32;
            let offset = (address & 0xFFFFFFFF) as u32;

            warn!(
                "SpiceAddress 0x{:x} is encoded: surface_id=0x{:x} ({}) offset=0x{:x} ({})",
                address, surface_id, surface_id, offset, offset
            );

            // For now, we don't have a surface cache implemented
            // In a full implementation, we would look up the surface/cache entry
            // and return the data from that surface at the given offset
            warn!("Encoded SpiceAddress not supported yet - no surface cache system");
            return None;
        }

        // Simple offset from message body
        let offset = address as usize;
        if offset >= data.len() {
            warn!(
                "SpiceAddress 0x{:x} ({}) is out of bounds (data len: {})",
                address,
                address,
                data.len()
            );
            return None;
        }

        Some(&data[offset..])
    }

    /// Decode a SpiceImage from a SpiceAddress
    fn decode_image(
        &mut self,
        address: SpiceAddress,
        data: &[u8],
    ) -> Result<Option<(Vec<u8>, u32, u32)>> {
        // Check if this is a special cached image address
        // SPICE uses high addresses (> 0x10000000) for special encodings
        if address > 0x10000000 {
            // This is likely a cached image reference
            // Extract the cache ID from the address encoding
            let cache_id = address & 0xFFFFFFFF; // Lower 32 bits might be the cache ID

            debug!(
                "Detected cached image reference with address 0x{:x}, cache_id: {}",
                address, cache_id
            );

            // Try to get from cache
            if let Some(cached) = self.image_cache.get(cache_id) {
                debug!("Found cached image: {}x{}", cached.width, cached.height);
                return Ok(Some((cached.data.clone(), cached.width, cached.height)));
            } else {
                warn!("Cached image with ID {} not found in cache", cache_id);
                return Ok(None);
            }
        }

        let image_data = match self.resolve_address(address, data) {
            Some(d) => d,
            None => return Ok(None),
        };

        // Parse the image descriptor
        let mut cursor = std::io::Cursor::new(image_data);
        let descriptor = SpiceImageDescriptor::read(&mut cursor)
            .map_err(|e| SpiceError::Protocol(format!("Failed to parse image descriptor: {e}")))?;

        debug!(
            "Image descriptor: type={}, size={}x{}, id={}",
            descriptor.type_, descriptor.width, descriptor.height, descriptor.id
        );

        let result = match descriptor.type_ {
            SPICE_IMAGE_TYPE_BITMAP => {
                // Parse bitmap structure
                let bitmap = SpiceBitmap::read(&mut cursor)
                    .map_err(|e| SpiceError::Protocol(format!("Failed to parse bitmap: {e}")))?;

                // Get bitmap data
                let bitmap_data_offset = bitmap.data as usize;
                if bitmap_data_offset >= data.len() {
                    warn!("Bitmap data offset {} out of bounds", bitmap_data_offset);
                    return Ok(None);
                }

                let bitmap_data = &data[bitmap_data_offset..];
                self.decode_bitmap(&bitmap, bitmap_data, descriptor.width, descriptor.height)
            }
            SPICE_IMAGE_TYPE_LZ4 => {
                // Decompress LZ4 data
                let compressed_data = &image_data[cursor.position() as usize..];
                self.decode_lz4(compressed_data, descriptor.width, descriptor.height)
            }
            SPICE_IMAGE_TYPE_JPEG => {
                // Decode JPEG data
                let jpeg_data = &image_data[cursor.position() as usize..];
                self.decode_jpeg(jpeg_data)
            }
            SPICE_IMAGE_TYPE_LZ => {
                // Decompress LZ data (SPICE custom LZ format)
                let compressed_data = &image_data[cursor.position() as usize..];
                self.decode_lz(compressed_data, descriptor.width, descriptor.height)
            }
            SPICE_IMAGE_TYPE_ZLIB_GLZ_RGB => {
                // Decompress zlib data
                let compressed_data = &image_data[cursor.position() as usize..];
                self.decode_zlib(compressed_data, descriptor.width, descriptor.height)
            }
            SPICE_IMAGE_TYPE_FROM_CACHE | SPICE_IMAGE_TYPE_FROM_CACHE_LOSSLESS => {
                // This is a cached image reference
                debug!("FROM_CACHE image type, id: {}", descriptor.id);

                // Try to get from cache
                if let Some(cached) = self.image_cache.get(descriptor.id) {
                    debug!("Found cached image: {}x{}", cached.width, cached.height);
                    Ok(Some((cached.data.clone(), cached.width, cached.height)))
                } else {
                    warn!("Cached image with ID {} not found in cache", descriptor.id);
                    Ok(None)
                }
            }
            SPICE_IMAGE_TYPE_SURFACE => {
                // This references another surface
                debug!(
                    "SURFACE image type, surface id encoded in descriptor id: {}",
                    descriptor.id
                );

                // The descriptor.id might encode the surface ID
                let surface_id = (descriptor.id & 0xFFFFFFFF) as u32;

                if let Some(surface) = self.surfaces.get(&surface_id) {
                    debug!(
                        "Found surface {}: {}x{}",
                        surface_id, surface.width, surface.height
                    );
                    Ok(Some((surface.data.clone(), surface.width, surface.height)))
                } else {
                    warn!("Surface {} not found", surface_id);
                    Ok(None)
                }
            }
            _ => {
                debug!("Unsupported image type: {}", descriptor.type_);
                Ok(None)
            }
        }?;

        // Cache the decoded image if successful (except for FROM_CACHE and SURFACE types)
        if let Some((ref data, width, height)) = result {
            if descriptor.type_ != SPICE_IMAGE_TYPE_FROM_CACHE
                && descriptor.type_ != SPICE_IMAGE_TYPE_FROM_CACHE_LOSSLESS
                && descriptor.type_ != SPICE_IMAGE_TYPE_SURFACE
            {
                debug!("Caching decoded image with id: {}", descriptor.id);
                self.image_cache
                    .insert(descriptor.id, data.clone(), width, height);
            }
        }

        Ok(result)
    }

    /// Decode a raw bitmap to RGBA format
    fn decode_bitmap(
        &self,
        bitmap: &SpiceBitmap,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Option<(Vec<u8>, u32, u32)>> {
        let bytes_per_pixel = match bitmap.format {
            SPICE_BITMAP_FMT_32BIT | SPICE_BITMAP_FMT_RGBA => 4,
            SPICE_BITMAP_FMT_24BIT => 3,
            SPICE_BITMAP_FMT_16BIT => 2,
            SPICE_BITMAP_FMT_8BIT | SPICE_BITMAP_FMT_8BIT_A => 1,
            _ => {
                warn!("Unsupported bitmap format: {}", bitmap.format);
                return Ok(None);
            }
        };

        let expected_size = (bitmap.stride * height) as usize;
        if data.len() < expected_size {
            warn!("Bitmap data too small: {} < {}", data.len(), expected_size);
            return Ok(None);
        }

        // Convert to RGBA
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            let row_offset = (y * bitmap.stride) as usize;
            for x in 0..width {
                let pixel_offset = row_offset + (x as usize * bytes_per_pixel);

                match bitmap.format {
                    SPICE_BITMAP_FMT_32BIT => {
                        // BGRA -> RGBA
                        rgba_data.push(data[pixel_offset + 2]); // R
                        rgba_data.push(data[pixel_offset + 1]); // G
                        rgba_data.push(data[pixel_offset]); // B
                        rgba_data.push(data[pixel_offset + 3]); // A
                    }
                    SPICE_BITMAP_FMT_RGBA => {
                        // Already RGBA
                        rgba_data.extend_from_slice(&data[pixel_offset..pixel_offset + 4]);
                    }
                    SPICE_BITMAP_FMT_24BIT => {
                        // BGR -> RGBA
                        rgba_data.push(data[pixel_offset + 2]); // R
                        rgba_data.push(data[pixel_offset + 1]); // G
                        rgba_data.push(data[pixel_offset]); // B
                        rgba_data.push(255); // A
                    }
                    _ => {
                        // Fallback for other formats
                        rgba_data.extend_from_slice(&[0, 0, 0, 255]);
                    }
                }
            }
        }

        Ok(Some((rgba_data, width, height)))
    }

    /// Decode LZ4 compressed image
    fn decode_lz4(
        &self,
        compressed_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Option<(Vec<u8>, u32, u32)>> {
        use lz4::Decoder;
        use std::io::Read;

        let mut decoder = Decoder::new(compressed_data)
            .map_err(|e| SpiceError::Protocol(format!("Failed to create LZ4 decoder: {e}")))?;

        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| SpiceError::Protocol(format!("Failed to decompress LZ4: {e}")))?;

        // Assume decompressed data is RGBA
        Ok(Some((decompressed, width, height)))
    }

    /// Decode JPEG image
    fn decode_jpeg(&self, jpeg_data: &[u8]) -> Result<Option<(Vec<u8>, u32, u32)>> {
        use jpeg_decoder::Decoder;

        let mut decoder = Decoder::new(jpeg_data);
        let pixels = decoder
            .decode()
            .map_err(|e| SpiceError::Protocol(format!("Failed to decode JPEG: {e}")))?;

        let info = decoder.info().unwrap();
        let width = info.width as u32;
        let height = info.height as u32;

        // Convert to RGBA if needed
        let rgba_data = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => {
                let mut rgba = Vec::with_capacity(pixels.len() * 4 / 3);
                for chunk in pixels.chunks(3) {
                    rgba.push(chunk[0]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[2]); // B
                    rgba.push(255); // A
                }
                rgba
            }
            jpeg_decoder::PixelFormat::L8 => {
                let mut rgba = Vec::with_capacity(pixels.len() * 4);
                for &gray in &pixels {
                    rgba.push(gray); // R
                    rgba.push(gray); // G
                    rgba.push(gray); // B
                    rgba.push(255); // A
                }
                rgba
            }
            _ => {
                warn!("Unsupported JPEG pixel format: {:?}", info.pixel_format);
                return Ok(None);
            }
        };

        Ok(Some((rgba_data, width, height)))
    }

    /// Decode LZ compressed image (SPICE custom LZ format)
    /// This is a simplified implementation - full LZ support would require implementing the SPICE LZ algorithm
    fn decode_lz(
        &self,
        _compressed_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Option<(Vec<u8>, u32, u32)>> {
        // TODO: Implement SPICE LZ decompression algorithm
        // For now, return None to fall back to test pattern
        warn!("LZ decompression not yet implemented");
        Ok(None)
    }

    /// Decode zlib compressed image
    fn decode_zlib(
        &self,
        compressed_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Option<(Vec<u8>, u32, u32)>> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let mut decoder = ZlibDecoder::new(compressed_data);
        let mut decompressed = Vec::new();

        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| SpiceError::Protocol(format!("Failed to decompress zlib: {e}")))?;

        // Assume decompressed data is RGBA
        Ok(Some((decompressed, width, height)))
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "DisplayChannel: Starting event loop for channel {}",
            self.connection.channel_id
        );
        eprintln!("DisplayChannel: Entering message read loop");
        loop {
            eprintln!("DisplayChannel: Waiting for message...");
            match self.connection.read_message().await {
                Ok((header, data)) => {
                    eprintln!("DisplayChannel: Got message!");
                    self.handle_message(&header, &data).await?;
                }
                Err(e) => {
                    eprintln!("DisplayChannel: Error reading message: {e}");
                    return Err(e);
                }
            }
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
            eprintln!(
                "DisplayChannel: Creating primary surface {}x{} format {}",
                width, height, format
            );
            self.surfaces.insert(
                0,
                DisplaySurface {
                    width,
                    height,
                    format,
                    data: vec![0; (width * height * 4) as usize], // Assuming 32-bit RGBA
                },
            );

            // Notify about primary surface
            self.notify_update(0);
        }

        Ok(())
    }

    async fn handle_draw_message(&mut self, msg_type: u16, data: &[u8]) -> Result<()> {
        match msg_type {
            x if x == DisplayChannelMessage::DrawFill as u16 => {
                debug!("Handle draw fill");

                // For now, just log that we received a DrawFill
                // TODO: Implement proper parsing once SpiceDrawFill has binrw support
                info!("Received DrawFill message ({} bytes)", data.len());

                // Basic implementation: fill primary surface with a test color
                if let Some(surface) = self.surfaces.get_mut(&0) {
                    // Fill with a test pattern to show something is happening
                    let bytes_per_pixel = 4;
                    let stride = surface.width as usize * bytes_per_pixel;

                    // Fill top 100 pixels with red to show we're receiving draw commands
                    for y in 0..100.min(surface.height as usize) {
                        for x in 0..surface.width as usize {
                            let offset = y * stride + x * bytes_per_pixel;
                            if offset + 4 <= surface.data.len() {
                                surface.data[offset] = 255; // R
                                surface.data[offset + 1] = 0; // G
                                surface.data[offset + 2] = 0; // B
                                surface.data[offset + 3] = 255; // A
                            }
                        }
                    }

                    // Notify that the surface was updated
                    self.notify_update(0);
                }
            }
            x if x == DisplayChannelMessage::DrawCopy as u16 => {
                debug!("Handle draw copy");

                // Log raw data for debugging
                if data.len() < 100 {
                    debug!("DrawCopy raw data (hex): {:02x?}", data);
                } else {
                    debug!(
                        "DrawCopy raw data first 100 bytes (hex): {:02x?}",
                        &data[..100]
                    );
                }

                // Parse the draw copy message
                let mut cursor = std::io::Cursor::new(data);
                if let Ok(draw_copy) = SpiceDrawCopy::read(&mut cursor) {
                    let surface_id = draw_copy.base.surface_id;
                    let bbox = &draw_copy.base.box_;
                    let src_area = &draw_copy.data.src_area;

                    info!("DrawCopy on surface {} - rect: ({},{}) to ({},{}) from src rect ({},{}) to ({},{}) src_image: 0x{:x}", 
                          surface_id, bbox.left, bbox.top, bbox.right, bbox.bottom,
                          src_area.left, src_area.top, src_area.right, src_area.bottom,
                          draw_copy.data.src_image);

                    // Try to decode the source image
                    // Note: decode_image now handles special cached image addresses internally
                    let decoded_image = self.decode_image(draw_copy.data.src_image, data)?;

                    if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                        match decoded_image {
                            Some((image_data, img_width, img_height)) => {
                                info!("Decoded image: {}x{}", img_width, img_height);

                                // Copy the decoded image to the surface
                                let bytes_per_pixel = 4;
                                let surface_stride = surface.width as usize * bytes_per_pixel;
                                let image_stride = img_width as usize * bytes_per_pixel;

                                // Calculate source rectangle bounds
                                let src_left = src_area.left.max(0) as usize;
                                let src_top = src_area.top.max(0) as usize;
                                let src_right = src_area.right.min(img_width as i32) as usize;
                                let src_bottom = src_area.bottom.min(img_height as i32) as usize;

                                // Calculate destination bounds
                                let dst_left = bbox.left.max(0) as usize;
                                let dst_top = bbox.top.max(0) as usize;
                                let dst_right = bbox.right.min(surface.width as i32) as usize;
                                let dst_bottom = bbox.bottom.min(surface.height as i32) as usize;

                                // Copy pixels from source to destination
                                let copy_width = (src_right - src_left).min(dst_right - dst_left);
                                let copy_height = (src_bottom - src_top).min(dst_bottom - dst_top);

                                for y in 0..copy_height {
                                    let src_y = src_top + y;
                                    let dst_y = dst_top + y;

                                    if src_y < img_height as usize
                                        && dst_y < surface.height as usize
                                    {
                                        let src_row_offset = src_y * image_stride;
                                        let dst_row_offset = dst_y * surface_stride;

                                        for x in 0..copy_width {
                                            let src_x = src_left + x;
                                            let dst_x = dst_left + x;

                                            if src_x < img_width as usize
                                                && dst_x < surface.width as usize
                                            {
                                                let src_offset =
                                                    src_row_offset + src_x * bytes_per_pixel;
                                                let dst_offset =
                                                    dst_row_offset + dst_x * bytes_per_pixel;

                                                if src_offset + 4 <= image_data.len()
                                                    && dst_offset + 4 <= surface.data.len()
                                                {
                                                    // Copy RGBA pixels
                                                    surface.data[dst_offset..dst_offset + 4]
                                                        .copy_from_slice(
                                                            &image_data[src_offset..src_offset + 4],
                                                        );
                                                }
                                            }
                                        }
                                    }
                                }

                                self.notify_update(surface_id);
                            }
                            None => {
                                warn!("Failed to decode image at address 0x{:x}, using blue test pattern", draw_copy.data.src_image);

                                // Fallback to blue test pattern
                                let bytes_per_pixel = 4;
                                let stride = surface.width as usize * bytes_per_pixel;

                                for y in bbox.top.max(0)..bbox.bottom.min(surface.height as i32) {
                                    let y = y as usize;
                                    for x in bbox.left.max(0)..bbox.right.min(surface.width as i32)
                                    {
                                        let x = x as usize;
                                        let offset = y * stride + x * bytes_per_pixel;
                                        if offset + 4 <= surface.data.len() {
                                            surface.data[offset] = 0; // R
                                            surface.data[offset + 1] = 0; // G
                                            surface.data[offset + 2] = 255; // B
                                            surface.data[offset + 3] = 255; // A
                                        }
                                    }
                                }

                                self.notify_update(surface_id);
                            }
                        }
                    }
                } else {
                    warn!("Failed to parse DrawCopy message");
                }
            }
            x if x == DisplayChannelMessage::DrawOpaque as u16 => {
                debug!("Handle draw opaque");

                // Parse the draw opaque message
                let mut cursor = std::io::Cursor::new(data);
                if let Ok(draw_opaque) = SpiceDrawOpaque::read(&mut cursor) {
                    let surface_id = draw_opaque.base.surface_id;
                    let bbox = &draw_opaque.base.box_;
                    let brush = &draw_opaque.data.brush;
                    let src_area = &draw_opaque.data.src_area;

                    info!("DrawOpaque on surface {} - rect: ({},{}) to ({},{}) brush type: {} src_image: 0x{:x}", 
                          surface_id, bbox.left, bbox.top, bbox.right, bbox.bottom,
                          brush.brush_type, draw_opaque.data.src_image);

                    // Try to decode the source image first (before getting mutable reference)
                    let decoded_image = if draw_opaque.data.src_image != 0 {
                        self.decode_image(draw_opaque.data.src_image, data)?
                    } else {
                        None
                    };

                    if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                        let bytes_per_pixel = 4;
                        let stride = surface.width as usize * bytes_per_pixel;

                        // First, fill with brush background if it's a solid color
                        if brush.brush_type == 1 {
                            // SOLID brush
                            let r = ((brush.color >> 16) & 0xFF) as u8;
                            let g = ((brush.color >> 8) & 0xFF) as u8;
                            let b = (brush.color & 0xFF) as u8;

                            // Fill the destination rectangle with brush color
                            for y in bbox.top.max(0)..bbox.bottom.min(surface.height as i32) {
                                let y = y as usize;
                                for x in bbox.left.max(0)..bbox.right.min(surface.width as i32) {
                                    let x = x as usize;
                                    let offset = y * stride + x * bytes_per_pixel;
                                    if offset + 4 <= surface.data.len() {
                                        surface.data[offset] = r;
                                        surface.data[offset + 1] = g;
                                        surface.data[offset + 2] = b;
                                        surface.data[offset + 3] = 255;
                                    }
                                }
                            }
                        }

                        // If there's a source image, overlay it on top
                        if let Some((image_data, img_width, img_height)) = decoded_image {
                            info!("Decoded opaque source image: {}x{}", img_width, img_height);

                            // Copy the decoded image to the surface
                            let image_stride = img_width as usize * bytes_per_pixel;

                            // Calculate source rectangle bounds
                            let src_left = src_area.left.max(0) as usize;
                            let src_top = src_area.top.max(0) as usize;
                            let src_right = src_area.right.min(img_width as i32) as usize;
                            let src_bottom = src_area.bottom.min(img_height as i32) as usize;

                            // Calculate destination bounds
                            let dst_left = bbox.left.max(0) as usize;
                            let dst_top = bbox.top.max(0) as usize;
                            let dst_right = bbox.right.min(surface.width as i32) as usize;
                            let dst_bottom = bbox.bottom.min(surface.height as i32) as usize;

                            // Copy pixels from source to destination
                            let copy_width = (src_right - src_left).min(dst_right - dst_left);
                            let copy_height = (src_bottom - src_top).min(dst_bottom - dst_top);

                            for y in 0..copy_height {
                                let src_y = src_top + y;
                                let dst_y = dst_top + y;

                                if src_y < img_height as usize && dst_y < surface.height as usize {
                                    let src_row_offset = src_y * image_stride;
                                    let dst_row_offset = dst_y * stride;

                                    for x in 0..copy_width {
                                        let src_x = src_left + x;
                                        let dst_x = dst_left + x;

                                        if src_x < img_width as usize
                                            && dst_x < surface.width as usize
                                        {
                                            let src_offset =
                                                src_row_offset + src_x * bytes_per_pixel;
                                            let dst_offset =
                                                dst_row_offset + dst_x * bytes_per_pixel;

                                            if src_offset + 4 <= image_data.len()
                                                && dst_offset + 4 <= surface.data.len()
                                            {
                                                // Copy RGBA pixels (opaque means we ignore alpha from source)
                                                surface.data[dst_offset] = image_data[src_offset];
                                                surface.data[dst_offset + 1] =
                                                    image_data[src_offset + 1];
                                                surface.data[dst_offset + 2] =
                                                    image_data[src_offset + 2];
                                                surface.data[dst_offset + 3] = 255;
                                                // Always opaque
                                            }
                                        }
                                    }
                                }
                            }
                        } else if brush.brush_type != 1 {
                            // No image and no solid brush, use green test pattern
                            for y in bbox.top.max(0)..bbox.bottom.min(surface.height as i32) {
                                let y = y as usize;
                                for x in bbox.left.max(0)..bbox.right.min(surface.width as i32) {
                                    let x = x as usize;
                                    let offset = y * stride + x * bytes_per_pixel;
                                    if offset + 4 <= surface.data.len() {
                                        surface.data[offset] = 0;
                                        surface.data[offset + 1] = 255;
                                        surface.data[offset + 2] = 0;
                                        surface.data[offset + 3] = 255;
                                    }
                                }
                            }
                        }

                        self.notify_update(surface_id);
                    }
                } else {
                    warn!("Failed to parse DrawOpaque message");
                }
            }
            x if x == DisplayChannelMessage::DrawBlend as u16 => {
                debug!("Handle draw blend");

                // Parse the draw blend message
                let mut cursor = std::io::Cursor::new(data);
                if let Ok(draw_blend) = SpiceDrawBlend::read(&mut cursor) {
                    let surface_id = draw_blend.base.surface_id;
                    let bbox = &draw_blend.base.box_;

                    info!(
                        "DrawBlend on surface {} - rect: ({},{}) to ({},{})",
                        surface_id, bbox.left, bbox.top, bbox.right, bbox.bottom
                    );

                    // For now, just fill with purple to show we're processing DrawBlend
                    if let Some(surface) = self.surfaces.get_mut(&surface_id) {
                        let bytes_per_pixel = 4;
                        let stride = surface.width as usize * bytes_per_pixel;

                        // Fill with purple for testing
                        for y in bbox.top.max(0)..bbox.bottom.min(surface.height as i32) {
                            let y = y as usize;
                            for x in bbox.left.max(0)..bbox.right.min(surface.width as i32) {
                                let x = x as usize;
                                let offset = y * stride + x * bytes_per_pixel;
                                if offset + 4 <= surface.data.len() {
                                    surface.data[offset] = 128; // R
                                    surface.data[offset + 1] = 0; // G
                                    surface.data[offset + 2] = 128; // B
                                    surface.data[offset + 3] = 255; // A
                                }
                            }
                        }

                        self.notify_update(surface_id);
                    }
                } else {
                    warn!("Failed to parse DrawBlend message");
                }
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
                let mut cursor = std::io::Cursor::new(data);
                let stream_create = SpiceStreamCreate::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse StreamCreate: {e}"))
                })?;

                info!(
                    "Created stream {} - codec: {}, dimensions: {}x{} -> {}x{}",
                    stream_create.id,
                    stream_create.codec_type,
                    stream_create.src_width,
                    stream_create.src_height,
                    stream_create.stream_width,
                    stream_create.stream_height
                );

                // Store stream info for processing subsequent data
                self.active_streams.insert(
                    stream_create.id,
                    StreamInfo {
                        id: stream_create.id,
                        codec_type: stream_create.codec_type,
                        width: stream_create.stream_width,
                        height: stream_create.stream_height,
                        dest_rect: stream_create.dest.clone(),
                    },
                );
            }
            x if x == DisplayChannelMessage::StreamData as u16 => {
                debug!("Handle stream data");
                let mut cursor = std::io::Cursor::new(data);
                let stream_data = SpiceStreamData::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse StreamData: {e}"))
                })?;

                debug!(
                    "Received {} bytes for stream {}",
                    stream_data.data_size, stream_data.id
                );

                // TODO: Decode stream data and apply to surface
                // This would involve:
                // 1. Finding the decoder for this stream ID
                // 2. Decoding the data based on codec type
                // 3. Applying decoded frame to the display surface
            }
            x if x == DisplayChannelMessage::StreamDestroy as u16 => {
                debug!("Handle stream destroy");
                let mut cursor = std::io::Cursor::new(data);
                let stream_destroy = SpiceStreamDestroy::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse StreamDestroy: {e}"))
                })?;

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
        eprintln!(
            "DisplayChannel: Received message type {} with {} bytes",
            header.msg_type,
            data.len()
        );

        // Log specific message types for debugging
        match header.msg_type {
            101 => eprintln!("  -> SPICE_MSG_DISPLAY_MODE"),
            318 => eprintln!("  -> SPICE_MSG_DISPLAY_SURFACE_CREATE"),
            319 => eprintln!("  -> SPICE_MSG_DISPLAY_SURFACE_DESTROY"),
            122 => eprintln!("  -> SPICE_MSG_DISPLAY_STREAM_CREATE"),
            123 => eprintln!("  -> SPICE_MSG_DISPLAY_STREAM_DATA"),
            125 => eprintln!("  -> SPICE_MSG_DISPLAY_STREAM_DESTROY"),
            302 => eprintln!("  -> SPICE_MSG_DISPLAY_DRAW_FILL"),
            304 => eprintln!("  -> SPICE_MSG_DISPLAY_DRAW_COPY"),
            _ => {}
        }

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
            x if (DisplayChannelMessage::DrawFill as u16
                ..=DisplayChannelMessage::DrawAlphaBlend as u16)
                .contains(&x) =>
            {
                self.handle_draw_message(header.msg_type, data).await?;
            }
            x if (DisplayChannelMessage::StreamCreate as u16
                ..=DisplayChannelMessage::StreamDestroyAll as u16)
                .contains(&x) =>
            {
                self.handle_stream_message(header.msg_type, data).await?;
            }
            x if x == SPICE_MSG_DISPLAY_SURFACE_CREATE => {
                debug!("Received surface create");
                let mut cursor = std::io::Cursor::new(data);
                let surface_create = SpiceMsgSurfaceCreate::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse SurfaceCreate: {e}"))
                })?;

                info!(
                    "Creating surface {} - {}x{} format: {}",
                    surface_create.surface_id,
                    surface_create.width,
                    surface_create.height,
                    surface_create.format
                );

                // Create new surface
                let data_size = (surface_create.width * surface_create.height * 4) as usize;
                self.surfaces.insert(
                    surface_create.surface_id,
                    DisplaySurface {
                        width: surface_create.width,
                        height: surface_create.height,
                        format: surface_create.format,
                        data: vec![0; data_size],
                    },
                );

                // Notify about new surface
                self.notify_update(surface_create.surface_id);
            }
            x if x == SPICE_MSG_DISPLAY_SURFACE_DESTROY => {
                debug!("Received surface destroy");
                let mut cursor = std::io::Cursor::new(data);
                let surface_destroy = SpiceMsgSurfaceDestroy::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse SurfaceDestroy: {e}"))
                })?;

                info!("Destroying surface {}", surface_destroy.surface_id);

                self.surfaces.remove(&surface_destroy.surface_id);
            }
            x if x == SPICE_MSG_DISPLAY_MONITORS_CONFIG => {
                debug!("Received monitors config");
                let mut cursor = std::io::Cursor::new(data);
                let monitors_config = SpiceMonitorsConfig::read(&mut cursor).map_err(|e| {
                    SpiceError::Protocol(format!("Failed to parse MonitorsConfig: {e}"))
                })?;

                info!(
                    "Monitors config: {} monitors (max {})",
                    monitors_config.count, monitors_config.max_allowed
                );

                self.monitors = monitors_config.heads;

                // Log monitor configurations
                for (i, head) in self.monitors.iter().enumerate() {
                    info!(
                        "Monitor {}: {}x{} at ({},{}) on surface {}",
                        i, head.width, head.height, head.x, head.y, head.surface_id
                    );
                }
            }
            3 => {
                // SPICE_MSG_SET_ACK
                eprintln!("DisplayChannel: Received SET_ACK message");
                // Parse the generation number
                if data.len() >= 4 {
                    let generation = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    eprintln!("DisplayChannel: SET_ACK generation: {generation}");

                    // Send ACK_SYNC response
                    let ack_data = generation.to_le_bytes();
                    self.connection.send_message(1, &ack_data).await?; // SPICE_MSGC_ACK_SYNC
                    eprintln!("DisplayChannel: Sent ACK_SYNC response");
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
