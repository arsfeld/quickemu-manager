use super::{MultimediaBackend, MultimediaError, Result};
use sdl2::Sdl;
use std::sync::{Arc, Mutex};

pub mod display;
pub mod audio;
pub mod input;

pub struct Sdl2Backend {
    sdl_context: Arc<Sdl>,
}

impl Sdl2Backend {
    pub fn new() -> Result<Self> {
        let sdl_context = sdl2::init()
            .map_err(|e| MultimediaError::new(format!("Failed to initialize SDL2: {}", e)))?;
        
        Ok(Self {
            sdl_context: Arc::new(sdl_context),
        })
    }
}

impl MultimediaBackend for Sdl2Backend {
    type Display = display::Sdl2Display;
    type Audio = audio::Sdl2Audio;
    type Input = input::Sdl2Input;

    fn create_display(&self) -> Result<Self::Display> {
        display::Sdl2Display::new(self.sdl_context.clone())
    }

    fn create_audio(&self) -> Result<Self::Audio> {
        audio::Sdl2Audio::new(self.sdl_context.clone())
    }

    fn create_input(&self) -> Result<Self::Input> {
        input::Sdl2Input::new(self.sdl_context.clone())
    }
}