use crate::channels::display::DisplaySurface;
use base64::{engine::general_purpose, Engine as _};
use instant::Instant;

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data_url: String,
    pub timestamp: Instant,
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
            timestamp: Instant::now(),
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