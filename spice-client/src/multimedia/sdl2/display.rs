use crate::multimedia::{
    display::{CursorData, Display, DisplayMode, PixelFormat},
    MultimediaError, Result,
};
use sdl2::{
    pixels::PixelFormatEnum,
    render::{Canvas, TextureCreator},
    video::{Window, WindowContext},
    Sdl,
};
use std::sync::Arc;

pub struct Sdl2Display {
    sdl_context: Arc<Sdl>,
    canvas: Option<Canvas<Window>>,
    texture_creator: Option<TextureCreator<WindowContext>>,
    dimensions: (u32, u32),
    fullscreen: bool,
    current_format: Option<PixelFormat>,
}

// Safety: SDL2 display is only accessed from the main thread
unsafe impl Send for Sdl2Display {}

impl Sdl2Display {
    pub fn new(sdl_context: Arc<Sdl>) -> Result<Self> {
        Ok(Self {
            sdl_context,
            canvas: None,
            texture_creator: None,
            dimensions: (0, 0),
            fullscreen: false,
            current_format: None,
        })
    }

    fn pixel_format_to_sdl(&self, format: PixelFormat) -> PixelFormatEnum {
        match format {
            PixelFormat::Rgb888 => PixelFormatEnum::RGB888,
            PixelFormat::Rgba8888 => PixelFormatEnum::RGBA8888,
            PixelFormat::Bgr888 => PixelFormatEnum::BGR888,
            PixelFormat::Bgra8888 => PixelFormatEnum::BGRA8888,
            PixelFormat::Rgb565 => PixelFormatEnum::RGB565,
        }
    }
}

impl Display for Sdl2Display {
    fn create_surface(&mut self, mode: DisplayMode) -> Result<()> {
        let video_subsystem = self.sdl_context.video()
            .map_err(|e| MultimediaError::new(format!("Failed to get video subsystem: {}", e)))?;

        let mut window_builder = video_subsystem.window("SPICE Client", mode.width, mode.height);
        window_builder.position_centered().resizable();

        if mode.fullscreen {
            window_builder.fullscreen_desktop();
        }

        let window = window_builder.build()
            .map_err(|e| MultimediaError::new(format!("Failed to create window: {}", e)))?;

        let canvas = window.into_canvas().accelerated().present_vsync().build()
            .map_err(|e| MultimediaError::new(format!("Failed to create canvas: {}", e)))?;

        let texture_creator = canvas.texture_creator();

        self.dimensions = (mode.width, mode.height);
        self.fullscreen = mode.fullscreen;
        self.texture_creator = Some(texture_creator);
        self.canvas = Some(canvas);

        Ok(())
    }

    fn present_frame(&mut self, data: &[u8], format: PixelFormat) -> Result<()> {
        let (width, height) = self.dimensions;
        let pixel_format = self.pixel_format_to_sdl(format);

        let texture_creator = self.texture_creator.as_ref()
            .ok_or_else(|| MultimediaError::new("Texture creator not initialized"))?;

        // Create texture for this frame
        let mut texture = texture_creator
            .create_texture_streaming(pixel_format, width, height)
            .map_err(|e| MultimediaError::new(format!("Failed to create texture: {}", e)))?;

        texture.update(None, data, (width * format.bytes_per_pixel() as u32) as usize)
            .map_err(|e| MultimediaError::new(format!("Failed to update texture: {}", e)))?;

        let canvas = self.canvas.as_mut()
            .ok_or_else(|| MultimediaError::new("Display not initialized"))?;

        canvas.clear();
        canvas.copy(&texture, None, None)
            .map_err(|e| MultimediaError::new(format!("Failed to copy texture: {}", e)))?;
        canvas.present();

        self.current_format = Some(format);

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.dimensions = (width, height);
        
        if let Some(canvas) = &mut self.canvas {
            let window = canvas.window_mut();
            window.set_size(width, height)
                .map_err(|e| MultimediaError::new(format!("Failed to resize window: {}", e)))?;
        }

        
        Ok(())
    }

    fn set_cursor(&mut self, cursor: Option<CursorData>) -> Result<()> {
        // SDL2 cursor handling would go here
        // For now, just use system cursor
        Ok(())
    }

    fn set_title(&mut self, title: &str) -> Result<()> {
        if let Some(canvas) = &mut self.canvas {
            let window = canvas.window_mut();
            window.set_title(title)
                .map_err(|e| MultimediaError::new(format!("Failed to set window title: {}", e)))?;
        }
        Ok(())
    }

    fn toggle_fullscreen(&mut self) -> Result<()> {
        if let Some(canvas) = &mut self.canvas {
            let window = canvas.window_mut();
            
            self.fullscreen = !self.fullscreen;
            
            let fullscreen_type = if self.fullscreen {
                sdl2::video::FullscreenType::Desktop
            } else {
                sdl2::video::FullscreenType::Off
            };
            
            window.set_fullscreen(fullscreen_type)
                .map_err(|e| MultimediaError::new(format!("Failed to toggle fullscreen: {}", e)))?;
        }
        Ok(())
    }

    fn get_dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}