use crate::channels::display::DisplaySurface;
use base64::{engine::general_purpose, Engine as _};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::RwLock;

#[cfg(target_arch = "wasm32")]
use {
    std::sync::Mutex,
    js_sys,
};

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data_url: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub timestamp: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    pub timestamp: f64, // Use JS timestamp for WASM
}

impl PartialEq for VideoFrame {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width 
            && self.height == other.height 
            && self.data_url == other.data_url
        // Note: We exclude timestamp from equality comparison
    }
}

impl VideoFrame {
    pub fn from_surface(surface: &DisplaySurface) -> Self {
        let data_url = Self::create_data_url(surface.width, surface.height, &surface.data, surface.format);
        
        Self {
            width: surface.width,
            height: surface.height,
            data_url,
            #[cfg(not(target_arch = "wasm32"))]
            timestamp: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            timestamp: js_sys::Date::now(),
        }
    }

    fn create_data_url(width: u32, height: u32, data: &[u8], format: u32) -> String {
        // Convert raw pixel data to a web-compatible format
        match format {
            32 => Self::rgba_to_data_url(width, height, data),
            24 => Self::rgb_to_data_url(width, height, data),
            _ => Self::create_placeholder_svg(width, height),
        }
    }

    fn rgba_to_data_url(width: u32, height: u32, data: &[u8]) -> String {
        // Create a simple bitmap for RGBA data
        // In a real implementation, you'd convert to PNG or JPEG
        if data.len() >= (width * height * 4) as usize {
            // For now, create a canvas-compatible data URL
            // This is a simplified approach - in production you'd use proper image encoding
            let encoded = general_purpose::STANDARD.encode(data);
            format!("data:image/rgba;base64,{}", encoded)
        } else {
            Self::create_placeholder_svg(width, height)
        }
    }

    fn rgb_to_data_url(width: u32, height: u32, data: &[u8]) -> String {
        // Convert RGB to RGBA for web compatibility
        if data.len() >= (width * height * 3) as usize {
            let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
            
            for i in (0..data.len()).step_by(3) {
                if i + 2 < data.len() {
                    rgba_data.push(data[i]);     // R
                    rgba_data.push(data[i + 1]); // G
                    rgba_data.push(data[i + 2]); // B
                    rgba_data.push(255);         // A (opaque)
                }
            }
            
            let encoded = general_purpose::STANDARD.encode(&rgba_data);
            format!("data:image/rgba;base64,{}", encoded)
        } else {
            Self::create_placeholder_svg(width, height)
        }
    }

    fn create_placeholder_svg(width: u32, height: u32) -> String {
        let svg = format!(
            r##"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
                <rect width="100%" height="100%" fill="#2d3748"/>
                <text x="50%" y="50%" font-family="Arial" font-size="16" fill="#e2e8f0" text-anchor="middle" dy=".3em">
                    SPICE Display {}x{}
                </text>
               </svg>"##,
            width, height, width, height
        );
        
        let encoded = general_purpose::STANDARD.encode(svg.as_bytes());
        format!("data:image/svg+xml;base64,{}", encoded)
    }
}

#[derive(Debug, Clone)]
#[cfg(not(target_arch = "wasm32"))]
pub struct VideoOutput {
    current_frame: Arc<RwLock<Option<VideoFrame>>>,
    frame_count: Arc<RwLock<u64>>,
}

#[cfg(target_arch = "wasm32")]
pub struct VideoOutput {
    current_frame: Arc<Mutex<Option<VideoFrame>>>,
    frame_count: Arc<Mutex<u64>>,
}

impl VideoOutput {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self {
            current_frame: Arc::new(RwLock::new(None)),
            frame_count: Arc::new(RwLock::new(0)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self {
            current_frame: Arc::new(Mutex::new(None)),
            frame_count: Arc::new(Mutex::new(0)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn update_frame(&self, surface: &DisplaySurface) {
        let frame = VideoFrame::from_surface(surface);
        *self.current_frame.write().await = Some(frame);
        *self.frame_count.write().await += 1;
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn update_frame(&self, surface: &DisplaySurface) {
        let frame = VideoFrame::from_surface(surface);
        if let Ok(mut current) = self.current_frame.lock() {
            *current = Some(frame);
        }
        if let Ok(mut count) = self.frame_count.lock() {
            *count += 1;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_current_frame(&self) -> Option<VideoFrame> {
        self.current_frame.read().await.clone()
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_current_frame(&self) -> Option<VideoFrame> {
        self.current_frame.lock().ok().and_then(|guard| guard.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_frame_count(&self) -> u64 {
        *self.frame_count.read().await
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn get_frame_count(&self) -> u64 {
        self.frame_count.lock().map(|guard| *guard).unwrap_or(0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn clear(&self) {
        *self.current_frame.write().await = None;
        *self.frame_count.write().await = 0;
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn clear(&self) {
        if let Ok(mut current) = self.current_frame.lock() {
            *current = None;
        }
        if let Ok(mut count) = self.frame_count.lock() {
            *count = 0;
        }
    }
}

impl Default for VideoOutput {
    fn default() -> Self {
        Self::new()
    }
}