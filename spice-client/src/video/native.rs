use super::{VideoFrame, VideoOutput};
use crate::channels::display::DisplaySurface;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct NativeVideoOutput {
    current_frame: Arc<RwLock<Option<VideoFrame>>>,
    frame_count: Arc<RwLock<u64>>,
}

impl NativeVideoOutput {
    pub fn new() -> Self {
        Self {
            current_frame: Arc::new(RwLock::new(None)),
            frame_count: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl VideoOutput for NativeVideoOutput {
    async fn update_frame(&self, surface: &DisplaySurface) {
        let frame = VideoFrame::from_surface(surface);
        *self.current_frame.write().await = Some(frame);
        *self.frame_count.write().await += 1;
    }

    async fn get_current_frame(&self) -> Option<VideoFrame> {
        self.current_frame.read().await.clone()
    }

    async fn get_frame_count(&self) -> u64 {
        *self.frame_count.read().await
    }
}
