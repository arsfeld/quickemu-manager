use super::{AudioSpec, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    U8,
    S16,
    S32,
    F32,
}

impl AudioFormat {
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::S16 => 2,
            Self::S32 | Self::F32 => 4,
        }
    }
}

pub trait AudioOutput {
    fn initialize(&mut self, spec: AudioSpec, format: AudioFormat) -> Result<()>;

    fn queue_samples(&mut self, samples: &[u8]) -> Result<()>;

    fn get_queued_size(&self) -> usize;

    fn clear_queue(&mut self) -> Result<()>;

    fn set_volume(&mut self, volume: f32) -> Result<()>;

    fn get_volume(&self) -> f32;

    fn pause(&mut self, paused: bool) -> Result<()>;

    fn is_paused(&self) -> bool;

    fn get_spec(&self) -> Option<&AudioSpec>;
}
