use super::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb888,
    Rgba8888,
    Bgr888,
    Bgra8888,
    Rgb565,
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgb888 | Self::Bgr888 => 3,
            Self::Rgba8888 | Self::Bgra8888 => 4,
            Self::Rgb565 => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CursorData {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub data: Vec<u8>,
    pub format: PixelFormat,
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            fullscreen: false,
        }
    }
}

pub trait Display {
    fn create_surface(&mut self, mode: DisplayMode) -> Result<()>;

    fn present_frame(&mut self, data: &[u8], format: PixelFormat) -> Result<()>;

    fn resize(&mut self, width: u32, height: u32) -> Result<()>;

    fn set_cursor(&mut self, cursor: Option<CursorData>) -> Result<()>;

    fn set_title(&mut self, title: &str) -> Result<()>;

    fn toggle_fullscreen(&mut self) -> Result<()>;

    fn get_dimensions(&self) -> (u32, u32);

    fn is_fullscreen(&self) -> bool;

    /// Get a reference to self as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get a mutable reference to self as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
