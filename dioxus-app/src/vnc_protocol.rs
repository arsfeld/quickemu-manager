use std::sync::Arc;
use tokio::sync::RwLock;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone)]
pub struct VncState {
    pub width: u32,
    pub height: u32,
    pub name: String,
    pub pixel_format: PixelFormat,
    pub framebuffer: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PixelFormat {
    pub bits_per_pixel: u8,
    pub depth: u8,
    pub big_endian: bool,
    pub true_color: bool,
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self {
            bits_per_pixel: 32,
            depth: 24,
            big_endian: false,
            true_color: true,
            red_max: 255,
            green_max: 255,
            blue_max: 255,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
        }
    }
}

pub struct VncProtocolHandler {
    state: Arc<RwLock<VncState>>,
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
}

impl VncProtocolHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(VncState {
                width: 800,
                height: 600,
                name: String::new(),
                pixel_format: PixelFormat::default(),
                framebuffer: vec![0; 800 * 600 * 4],
            })),
            canvas: None,
            context: None,
        }
    }
    
    pub fn set_canvas(&mut self, container_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
        
        // Get the container div
        let container = document
            .get_element_by_id(container_id)
            .ok_or_else(|| JsValue::from_str("Container not found"))?;
        
        // Create a canvas element
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        
        canvas.set_width(800);
        canvas.set_height(600);
        canvas.set_class_name("vnc-canvas");
        canvas.style().set_property("width", "100%")?;
        canvas.style().set_property("height", "100%")?;
        
        // Clear container and append canvas
        container.set_inner_html("");
        container.append_child(&canvas)?;
        
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Failed to get 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        
        self.canvas = Some(canvas);
        self.context = Some(context);
        
        Ok(())
    }
    
    pub async fn handle_server_message(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let msg_type = data[0];
        match msg_type {
            0 => self.handle_framebuffer_update(&data[1..]).await,
            1 => self.handle_set_color_map_entries(&data[1..]).await,
            2 => self.handle_bell(&data[1..]).await,
            3 => self.handle_server_cut_text(&data[1..]).await,
            _ => {
                log::warn!("Unknown server message type: {}", msg_type);
                Ok(Vec::new())
            }
        }
    }
    
    async fn handle_framebuffer_update(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 3 {
            return Err("Framebuffer update too short".to_string());
        }
        
        let _padding = data[0];
        let num_rectangles = u16::from_be_bytes([data[1], data[2]]);
        
        let mut offset = 3;
        for _ in 0..num_rectangles {
            if offset + 12 > data.len() {
                return Err("Rectangle header incomplete".to_string());
            }
            
            let x = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let y = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            let width = u16::from_be_bytes([data[offset + 4], data[offset + 5]]);
            let height = u16::from_be_bytes([data[offset + 6], data[offset + 7]]);
            let encoding = i32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]]);
            
            offset += 12;
            
            match encoding {
                0 => {
                    // Raw encoding
                    let pixel_size = self.state.read().await.pixel_format.bits_per_pixel as usize / 8;
                    let rect_size = (width as usize) * (height as usize) * pixel_size;
                    
                    if offset + rect_size > data.len() {
                        return Err("Raw rectangle data incomplete".to_string());
                    }
                    
                    self.update_framebuffer(x, y, width, height, &data[offset..offset + rect_size]).await?;
                    offset += rect_size;
                }
                _ => {
                    log::warn!("Unsupported encoding: {}", encoding);
                    return Err(format!("Unsupported encoding: {}", encoding));
                }
            }
        }
        
        // Request next frame
        Ok(self.framebuffer_update_request(false, 0, 0, self.state.read().await.width as u16, self.state.read().await.height as u16))
    }
    
    async fn update_framebuffer(&mut self, x: u16, y: u16, width: u16, height: u16, data: &[u8]) -> Result<(), String> {
        let state = self.state.read().await;
        let fb_width = state.width as usize;
        
        // Convert pixel data and update canvas
        if let (Some(canvas), Some(context)) = (&self.canvas, &self.context) {
            // Create ImageData for this rectangle
            let mut image_data = vec![0u8; (width as usize) * (height as usize) * 4];
            
            let pixel_size = state.pixel_format.bits_per_pixel as usize / 8;
            for row in 0..height as usize {
                for col in 0..width as usize {
                    let src_offset = (row * width as usize + col) * pixel_size;
                    let dst_offset = (row * width as usize + col) * 4;
                    
                    if pixel_size == 4 {
                        // 32-bit RGBA
                        image_data[dst_offset] = data[src_offset + 2];     // R
                        image_data[dst_offset + 1] = data[src_offset + 1]; // G
                        image_data[dst_offset + 2] = data[src_offset];     // B
                        image_data[dst_offset + 3] = 255;                  // A
                    } else if pixel_size == 3 {
                        // 24-bit RGB
                        image_data[dst_offset] = data[src_offset + 2];     // R
                        image_data[dst_offset + 1] = data[src_offset + 1]; // G
                        image_data[dst_offset + 2] = data[src_offset];     // B
                        image_data[dst_offset + 3] = 255;                  // A
                    }
                }
            }
            
            // Draw pixels directly to canvas using fillRect for simplicity
            // This is not the most efficient method but avoids ImageData complexities
            for row in 0..height as usize {
                for col in 0..width as usize {
                    let src_idx = (row * width as usize + col) * 4;
                    let r = image_data[src_idx];
                    let g = image_data[src_idx + 1];
                    let b = image_data[src_idx + 2];
                    
                    let color = format!("rgb({},{},{})", r, g, b);
                    context.set_fill_style(&JsValue::from_str(&color));
                    context.fill_rect(
                        (x as usize + col) as f64,
                        (y as usize + row) as f64,
                        1.0,
                        1.0
                    );
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_set_color_map_entries(&mut self, _data: &[u8]) -> Result<Vec<u8>, String> {
        // Not implemented for true color
        Ok(Vec::new())
    }
    
    async fn handle_bell(&mut self, _data: &[u8]) -> Result<Vec<u8>, String> {
        log::info!("VNC Bell");
        Ok(Vec::new())
    }
    
    async fn handle_server_cut_text(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 7 {
            return Err("Server cut text too short".to_string());
        }
        
        let _padding = &data[0..3];
        let length = u32::from_be_bytes([data[3], data[4], data[5], data[6]]);
        
        if data.len() < 7 + length as usize {
            return Err("Server cut text incomplete".to_string());
        }
        
        let text = String::from_utf8_lossy(&data[7..7 + length as usize]);
        log::info!("Server cut text: {}", text);
        
        Ok(Vec::new())
    }
    
    // Client messages
    pub fn set_pixel_format(&self, pixel_format: &PixelFormat) -> Vec<u8> {
        let mut msg = vec![0; 20]; // SetPixelFormat message
        msg[0] = 0; // Message type
        msg[1] = 0; // Padding
        msg[2] = 0; // Padding
        msg[3] = 0; // Padding
        
        msg[4] = pixel_format.bits_per_pixel;
        msg[5] = pixel_format.depth;
        msg[6] = if pixel_format.big_endian { 1 } else { 0 };
        msg[7] = if pixel_format.true_color { 1 } else { 0 };
        
        msg[8] = (pixel_format.red_max >> 8) as u8;
        msg[9] = (pixel_format.red_max & 0xFF) as u8;
        msg[10] = (pixel_format.green_max >> 8) as u8;
        msg[11] = (pixel_format.green_max & 0xFF) as u8;
        msg[12] = (pixel_format.blue_max >> 8) as u8;
        msg[13] = (pixel_format.blue_max & 0xFF) as u8;
        
        msg[14] = pixel_format.red_shift;
        msg[15] = pixel_format.green_shift;
        msg[16] = pixel_format.blue_shift;
        
        msg
    }
    
    pub fn set_encodings(&self, encodings: &[i32]) -> Vec<u8> {
        let mut msg = vec![0; 4 + encodings.len() * 4];
        msg[0] = 2; // SetEncodings message type
        msg[1] = 0; // Padding
        
        let num_encodings = encodings.len() as u16;
        msg[2] = (num_encodings >> 8) as u8;
        msg[3] = (num_encodings & 0xFF) as u8;
        
        for (i, &encoding) in encodings.iter().enumerate() {
            let offset = 4 + i * 4;
            let bytes = encoding.to_be_bytes();
            msg[offset..offset + 4].copy_from_slice(&bytes);
        }
        
        msg
    }
    
    pub fn framebuffer_update_request(&self, incremental: bool, x: u16, y: u16, width: u16, height: u16) -> Vec<u8> {
        let mut msg = vec![0; 10];
        msg[0] = 3; // FramebufferUpdateRequest message type
        msg[1] = if incremental { 1 } else { 0 };
        
        msg[2] = (x >> 8) as u8;
        msg[3] = (x & 0xFF) as u8;
        msg[4] = (y >> 8) as u8;
        msg[5] = (y & 0xFF) as u8;
        msg[6] = (width >> 8) as u8;
        msg[7] = (width & 0xFF) as u8;
        msg[8] = (height >> 8) as u8;
        msg[9] = (height & 0xFF) as u8;
        
        msg
    }
    
    pub fn key_event(&self, down: bool, key: u32) -> Vec<u8> {
        let mut msg = vec![0; 8];
        msg[0] = 4; // KeyEvent message type
        msg[1] = if down { 1 } else { 0 };
        msg[2] = 0; // Padding
        msg[3] = 0; // Padding
        
        let key_bytes = key.to_be_bytes();
        msg[4..8].copy_from_slice(&key_bytes);
        
        msg
    }
    
    pub fn pointer_event(&self, button_mask: u8, x: u16, y: u16) -> Vec<u8> {
        let mut msg = vec![0; 6];
        msg[0] = 5; // PointerEvent message type
        msg[1] = button_mask;
        
        msg[2] = (x >> 8) as u8;
        msg[3] = (x & 0xFF) as u8;
        msg[4] = (y >> 8) as u8;
        msg[5] = (y & 0xFF) as u8;
        
        msg
    }
    
    pub fn client_cut_text(&self, text: &str) -> Vec<u8> {
        let text_bytes = text.as_bytes();
        let mut msg = vec![0; 8 + text_bytes.len()];
        
        msg[0] = 6; // ClientCutText message type
        msg[1] = 0; // Padding
        msg[2] = 0; // Padding
        msg[3] = 0; // Padding
        
        let length = text_bytes.len() as u32;
        let length_bytes = length.to_be_bytes();
        msg[4..8].copy_from_slice(&length_bytes);
        
        msg[8..].copy_from_slice(text_bytes);
        
        msg
    }
}