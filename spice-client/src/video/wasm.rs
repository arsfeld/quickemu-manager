use super::{VideoFrame, VideoOutput};
use crate::channels::display::DisplaySurface;
use std::sync::{Arc, Mutex};

pub struct WasmVideoOutput {
    current_frame: Arc<Mutex<Option<VideoFrame>>>,
    frame_count: Arc<Mutex<u64>>,
}

impl WasmVideoOutput {
    pub fn new() -> Self {
        Self {
            current_frame: Arc::new(Mutex::new(None)),
            frame_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl VideoOutput for WasmVideoOutput {
    async fn update_frame(&self, surface: &DisplaySurface) {
        let frame = VideoFrame::from_surface(surface);
        if let Ok(mut current) = self.current_frame.lock() {
            *current = Some(frame);
        }
        if let Ok(mut count) = self.frame_count.lock() {
            *count += 1;
        }
    }

    async fn get_current_frame(&self) -> Option<VideoFrame> {
        self.current_frame.lock().ok()?.clone()
    }

    async fn get_frame_count(&self) -> u64 {
        self.frame_count.lock().ok().map(|c| *c).unwrap_or(0)
    }
}