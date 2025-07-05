use super::VideoFrame;
use crate::channels::display::DisplaySurface;
use std::sync::Arc;

/// Trait for video output handling
#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
#[allow(async_fn_in_trait)]
pub trait VideoOutput: Send + Sync {
    /// Update the current frame
    async fn update_frame(&self, surface: &DisplaySurface);

    /// Get the current frame
    async fn get_current_frame(&self) -> Option<VideoFrame>;

    /// Get the total frame count
    async fn get_frame_count(&self) -> u64;
}

/// Trait for video output handling - WASM version
#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
#[allow(async_fn_in_trait)]
pub trait VideoOutput {
    /// Update the current frame
    async fn update_frame(&self, surface: &DisplaySurface);

    /// Get the current frame
    async fn get_current_frame(&self) -> Option<VideoFrame>;

    /// Get the total frame count
    async fn get_frame_count(&self) -> u64;
}

/// Create a platform-specific VideoOutput implementation
pub fn create_video_output() -> Arc<dyn VideoOutput> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Arc::new(super::native::NativeVideoOutput::new())
    }

    #[cfg(target_arch = "wasm32")]
    {
        Arc::new(super::wasm::WasmVideoOutput::new())
    }
}
