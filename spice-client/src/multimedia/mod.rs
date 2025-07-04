use crate::error::SpiceError;
use std::error::Error;
use std::fmt;

pub mod audio;
pub mod display;
pub mod input;
pub mod spice_adapter;

#[cfg(test)]
mod tests;

#[cfg(feature = "backend-gtk4")]
pub mod gtk4;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[derive(Debug)]
pub struct MultimediaError {
    message: String,
}

impl fmt::Display for MultimediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Multimedia error: {}", self.message)
    }
}

impl Error for MultimediaError {}

impl MultimediaError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<SpiceError> for MultimediaError {
    fn from(err: SpiceError) -> Self {
        Self {
            message: format!("SPICE error: {}", err),
        }
    }
}

pub type Result<T> = std::result::Result<T, MultimediaError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioSpec {
    pub frequency: u32,
    pub channels: u8,
    pub samples: u16,
}

impl Default for AudioSpec {
    fn default() -> Self {
        Self {
            frequency: 44100,
            channels: 2,
            samples: 4096,
        }
    }
}

pub trait MultimediaBackend {
    type Display: display::Display + Send;
    type Audio: audio::AudioOutput + Send;
    type Input: input::InputHandler + Send;

    fn create_display(&self) -> Result<Self::Display>;
    fn create_audio(&self) -> Result<Self::Audio>;
    fn create_input(&self) -> Result<Self::Input>;
}

#[cfg(feature = "backend-gtk4")]
pub fn create_default_backend() -> Result<impl MultimediaBackend> {
    gtk4::Gtk4Backend::new()
}

#[cfg(all(target_arch = "wasm32", not(feature = "backend-gtk4")))]
pub fn create_default_backend() -> Result<impl MultimediaBackend> {
    wasm::WasmBackend::new()
}
