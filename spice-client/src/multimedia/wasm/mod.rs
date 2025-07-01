use super::{MultimediaBackend, MultimediaError, Result};

pub mod display;
pub mod audio;
pub mod input;

pub struct WasmBackend;

impl WasmBackend {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl MultimediaBackend for WasmBackend {
    type Display = display::WasmDisplay;
    type Audio = audio::WasmAudio;
    type Input = input::WasmInput;

    fn create_display(&self) -> Result<Self::Display> {
        display::WasmDisplay::new()
    }

    fn create_audio(&self) -> Result<Self::Audio> {
        audio::WasmAudio::new()
    }

    fn create_input(&self) -> Result<Self::Input> {
        input::WasmInput::new()
    }
}