use crate::multimedia::{
    display::{CursorData, Display, DisplayMode, PixelFormat},
    MultimediaError, Result,
};

pub struct WasmDisplay {
    dimensions: (u32, u32),
    fullscreen: bool,
}

impl WasmDisplay {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dimensions: (0, 0),
            fullscreen: false,
        })
    }
}

impl Display for WasmDisplay {
    fn create_surface(&mut self, mode: DisplayMode) -> Result<()> {
        self.dimensions = (mode.width, mode.height);
        self.fullscreen = mode.fullscreen;
        // TODO: Create canvas element and setup WebGL/2D context
        Ok(())
    }

    fn present_frame(&mut self, _data: &[u8], _format: PixelFormat) -> Result<()> {
        // TODO: Draw to canvas using ImageData or WebGL texture
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.dimensions = (width, height);
        // TODO: Resize canvas element
        Ok(())
    }

    fn set_cursor(&mut self, _cursor: Option<CursorData>) -> Result<()> {
        // TODO: Set custom cursor using CSS
        Ok(())
    }

    fn set_title(&mut self, _title: &str) -> Result<()> {
        // TODO: Set document title
        Ok(())
    }

    fn toggle_fullscreen(&mut self) -> Result<()> {
        self.fullscreen = !self.fullscreen;
        // TODO: Use Fullscreen API
        Ok(())
    }

    fn get_dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }
}