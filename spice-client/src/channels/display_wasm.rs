//! WebAssembly-specific display channel integration

use crate::channels::display::{DisplayChannel, DisplaySurface};
use crate::error::Result;
use crate::protocol::*;
#[cfg(target_arch = "wasm32")]
use crate::wasm::canvas::CanvasManager;
#[cfg(target_arch = "wasm32")]
use crate::wasm::video_renderer::{WasmPerformanceOptimizer, WasmVideoRenderer};

#[cfg(target_arch = "wasm32")]
impl DisplayChannel {
    /// Creates a WebAssembly-compatible display channel with canvas support
    pub async fn new_wasm(websocket_url: &str, channel_id: u8) -> Result<Self> {
        Self::new_websocket(websocket_url, channel_id).await
    }

    /// Renders the display to a canvas element
    pub async fn render_to_canvas(&self, canvas_manager: &mut CanvasManager) -> Result<()> {
        // Render each surface to its corresponding canvas
        for (surface_id, surface) in self.get_surfaces() {
            canvas_manager.get_or_create_canvas(*surface_id, surface.width, surface.height)?;
            canvas_manager.update_canvas(*surface_id, surface)?;
        }

        // Arrange canvases based on monitor configuration
        canvas_manager.arrange_canvases(self.get_monitors())?;

        Ok(())
    }

    /// Renders a specific region to canvas (optimized for partial updates)
    pub async fn render_region_to_canvas(
        &self,
        canvas_manager: &CanvasManager,
        surface_id: u32,
        rect: &SpiceRect,
    ) -> Result<()> {
        if let Some(surface) = self.get_surface(surface_id) {
            let x = rect.left;
            let y = rect.top;
            let width = (rect.right - rect.left) as u32;
            let height = (rect.bottom - rect.top) as u32;

            canvas_manager.update_canvas_region(surface_id, surface, x, y, width, height)?;
        }

        Ok(())
    }
}

/// WebAssembly display manager that coordinates rendering
#[cfg(target_arch = "wasm32")]
pub struct WasmDisplayManager {
    canvas_manager: CanvasManager,
    video_renderer: WasmVideoRenderer,
    performance_optimizer: WasmPerformanceOptimizer,
}

#[cfg(target_arch = "wasm32")]
impl WasmDisplayManager {
    pub fn new() -> Self {
        Self {
            canvas_manager: CanvasManager::new(),
            video_renderer: WasmVideoRenderer::new(),
            performance_optimizer: WasmPerformanceOptimizer::new(),
        }
    }

    /// Handles a display update from the channel
    pub async fn handle_display_update(&mut self, channel: &DisplayChannel) -> Result<()> {
        // Skip frame if performance is poor
        if self.performance_optimizer.should_skip_frame() {
            return Ok(());
        }

        // Render all surfaces
        channel.render_to_canvas(&mut self.canvas_manager).await?;

        Ok(())
    }

    /// Handles video stream creation
    pub async fn handle_stream_create(
        &mut self,
        stream_info: &crate::channels::display::StreamInfo,
    ) -> Result<()> {
        self.video_renderer.create_video_stream(stream_info)?;

        // Position video element based on destination rectangle
        self.video_renderer.position_video(
            stream_info.id,
            stream_info.dest_rect.left,
            stream_info.dest_rect.top,
        )?;

        Ok(())
    }

    /// Handles video stream data
    pub async fn handle_stream_data(&mut self, stream_id: u32, data: &[u8]) -> Result<()> {
        // Apply quality adjustment based on performance
        let quality = self.performance_optimizer.get_quality_level();

        if quality < 100 {
            // TODO: Implement quality reduction (e.g., downsampling)
        }

        self.video_renderer.append_video_data(stream_id, data)?;
        Ok(())
    }

    /// Handles video stream destruction
    pub async fn handle_stream_destroy(&mut self, stream_id: u32) -> Result<()> {
        self.video_renderer.remove_video_stream(stream_id)?;
        Ok(())
    }

    /// Handles surface destruction
    pub async fn handle_surface_destroy(&mut self, surface_id: u32) -> Result<()> {
        self.canvas_manager.remove_canvas(surface_id)?;
        Ok(())
    }

    /// Gets all canvas elements for embedding in the DOM
    pub fn get_canvases(&self) -> Vec<&web_sys::HtmlCanvasElement> {
        self.canvas_manager.get_all_canvases()
    }
}
