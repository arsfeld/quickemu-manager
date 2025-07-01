use super::{MultimediaBackend, MultimediaError, Result};
use gtk4::{glib, prelude::*};
use std::sync::{Arc, Mutex};

pub mod display;
pub mod audio;
pub mod input;

pub struct Gtk4Backend {
    initialized: Arc<Mutex<bool>>,
}

impl Gtk4Backend {
    pub fn new() -> Result<Self> {
        // Ensure GTK is initialized
        if !gtk4::is_initialized() {
            gtk4::init().map_err(|e| MultimediaError::new(format!("Failed to initialize GTK4: {}", e)))?;
        }
        
        Ok(Self {
            initialized: Arc::new(Mutex::new(true)),
        })
    }
}

impl MultimediaBackend for Gtk4Backend {
    type Display = display::Gtk4Display;
    type Audio = audio::Gtk4Audio;
    type Input = input::Gtk4Input;

    fn create_display(&self) -> Result<Self::Display> {
        display::Gtk4Display::new()
    }

    fn create_audio(&self) -> Result<Self::Audio> {
        audio::Gtk4Audio::new()
    }

    fn create_input(&self) -> Result<Self::Input> {
        input::Gtk4Input::new()
    }
}