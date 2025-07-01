use crate::multimedia::{
    audio::{AudioFormat, AudioOutput},
    AudioSpec, MultimediaError, Result,
};
use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
    Sdl,
};
use std::sync::{Arc, Mutex};

struct AudioQueueCallback {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl AudioCallback for AudioQueueCallback {
    type Channel = u8;

    fn callback(&mut self, out: &mut [u8]) {
        let mut buffer = self.buffer.lock().unwrap();
        let len = out.len().min(buffer.len());
        
        if len > 0 {
            out[..len].copy_from_slice(&buffer[..len]);
            buffer.drain(..len);
        }
        
        // Fill rest with silence
        for byte in &mut out[len..] {
            *byte = 0;
        }
    }
}

pub struct Sdl2Audio {
    sdl_context: Arc<Sdl>,
    device: Option<AudioDevice<AudioQueueCallback>>,
    buffer: Arc<Mutex<Vec<u8>>>,
    spec: Option<AudioSpec>,
    format: Option<AudioFormat>,
    volume: f32,
    paused: bool,
}

// Safety: SDL2 audio is thread-safe when accessed properly
unsafe impl Send for Sdl2Audio {}

impl Sdl2Audio {
    pub fn new(sdl_context: Arc<Sdl>) -> Result<Self> {
        Ok(Self {
            sdl_context,
            device: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            spec: None,
            format: None,
            volume: 1.0,
            paused: false,
        })
    }

    fn format_to_sdl(&self, format: AudioFormat) -> sdl2::audio::AudioFormat {
        match format {
            AudioFormat::U8 => sdl2::audio::AudioFormat::U8,
            AudioFormat::S16 => sdl2::audio::AudioFormat::S16LSB,
            AudioFormat::S32 => sdl2::audio::AudioFormat::S32LSB,
            AudioFormat::F32 => sdl2::audio::AudioFormat::F32LSB,
        }
    }
}

impl AudioOutput for Sdl2Audio {
    fn initialize(&mut self, spec: AudioSpec, format: AudioFormat) -> Result<()> {
        let audio_subsystem = self.sdl_context.audio()
            .map_err(|e| MultimediaError::new(format!("Failed to get audio subsystem: {}", e)))?;

        let desired_spec = AudioSpecDesired {
            freq: Some(spec.frequency as i32),
            channels: Some(spec.channels),
            samples: Some(spec.samples),
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |_| {
                AudioQueueCallback {
                    buffer: self.buffer.clone(),
                }
            })
            .map_err(|e| MultimediaError::new(format!("Failed to open audio device: {}", e)))?;

        device.resume();

        self.device = Some(device);
        self.spec = Some(spec);
        self.format = Some(format);
        self.paused = false;

        Ok(())
    }

    fn queue_samples(&mut self, samples: &[u8]) -> Result<()> {
        if self.device.is_none() {
            return Err(MultimediaError::new("Audio device not initialized"));
        }

        let mut buffer = self.buffer.lock().unwrap();
        
        // Apply volume
        if self.volume != 1.0 {
            let mut processed = samples.to_vec();
            match self.format {
                Some(AudioFormat::S16) => {
                    for chunk in processed.chunks_exact_mut(2) {
                        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                        let adjusted = (sample as f32 * self.volume) as i16;
                        chunk.copy_from_slice(&adjusted.to_le_bytes());
                    }
                }
                Some(AudioFormat::F32) => {
                    for chunk in processed.chunks_exact_mut(4) {
                        let mut bytes = [0u8; 4];
                        bytes.copy_from_slice(chunk);
                        let sample = f32::from_le_bytes(bytes);
                        let adjusted = sample * self.volume;
                        chunk.copy_from_slice(&adjusted.to_le_bytes());
                    }
                }
                _ => {}
            }
            buffer.extend_from_slice(&processed);
        } else {
            buffer.extend_from_slice(samples);
        }

        Ok(())
    }

    fn get_queued_size(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    fn clear_queue(&mut self) -> Result<()> {
        self.buffer.lock().unwrap().clear();
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_volume(&self) -> f32 {
        self.volume
    }

    fn pause(&mut self, paused: bool) -> Result<()> {
        if let Some(device) = &self.device {
            if paused {
                device.pause();
            } else {
                device.resume();
            }
            self.paused = paused;
        }
        Ok(())
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn get_spec(&self) -> Option<&AudioSpec> {
        self.spec.as_ref()
    }
}