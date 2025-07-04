//! Canvas integration for WebAssembly rendering

use crate::channels::display::DisplaySurface;
use crate::error::{Result, SpiceError};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Manages HTML5 Canvas elements for rendering SPICE displays
pub struct CanvasManager {
    canvases: HashMap<u32, CanvasDisplay>,
}

struct CanvasDisplay {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    surface_id: u32,
}

impl CanvasManager {
    /// Creates a new canvas manager
    pub fn new() -> Self {
        Self {
            canvases: HashMap::new(),
        }
    }

    /// Creates or gets a canvas for a specific display
    pub fn get_or_create_canvas(
        &mut self,
        surface_id: u32,
        width: u32,
        height: u32,
    ) -> Result<&HtmlCanvasElement> {
        if !self.canvases.contains_key(&surface_id) {
            self.create_canvas(surface_id, width, height)?;
        }

        Ok(&self.canvases.get(&surface_id).unwrap().canvas)
    }

    /// Creates a new canvas for a display
    fn create_canvas(&mut self, surface_id: u32, width: u32, height: u32) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object available".to_string()))?;

        let document = window
            .document()
            .ok_or_else(|| SpiceError::Protocol("No document object available".to_string()))?;

        let canvas = document
            .create_element("canvas")
            .map_err(|_| SpiceError::Protocol("Failed to create canvas element".to_string()))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| SpiceError::Protocol("Failed to cast to HtmlCanvasElement".to_string()))?;

        canvas.set_width(width);
        canvas.set_height(height);
        canvas.set_id(&format!("spice-display-{}", surface_id));

        // Add CSS class for styling
        canvas.set_class_name("spice-display-canvas");

        let context = canvas
            .get_context("2d")
            .map_err(|_| SpiceError::Protocol("Failed to get 2D context".to_string()))?
            .ok_or_else(|| SpiceError::Protocol("No 2D context available".to_string()))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| {
                SpiceError::Protocol("Failed to cast to CanvasRenderingContext2d".to_string())
            })?;

        // Configure context for better performance
        context.set_image_smoothing_enabled(false);

        self.canvases.insert(
            surface_id,
            CanvasDisplay {
                canvas,
                context,
                surface_id,
            },
        );

        Ok(())
    }

    /// Updates a canvas with surface data
    pub fn update_canvas(&self, surface_id: u32, surface: &DisplaySurface) -> Result<()> {
        let display = self.canvases.get(&surface_id).ok_or_else(|| {
            SpiceError::Protocol(format!("Canvas not found for surface {}", surface_id))
        })?;

        // Ensure canvas size matches surface
        if display.canvas.width() != surface.width || display.canvas.height() != surface.height {
            display.canvas.set_width(surface.width);
            display.canvas.set_height(surface.height);
        }

        // Create ImageData from surface data
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&surface.data),
            surface.width,
            surface.height,
        )
        .map_err(|_| SpiceError::Protocol("Failed to create ImageData".to_string()))?;

        // Draw to canvas
        display
            .context
            .put_image_data(&image_data, 0.0, 0.0)
            .map_err(|_| SpiceError::Protocol("Failed to put image data".to_string()))?;

        Ok(())
    }

    /// Updates a region of the canvas
    pub fn update_canvas_region(
        &self,
        surface_id: u32,
        surface: &DisplaySurface,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let display = self.canvases.get(&surface_id).ok_or_else(|| {
            SpiceError::Protocol(format!("Canvas not found for surface {}", surface_id))
        })?;

        // Extract the region data
        let mut region_data = Vec::with_capacity((width * height * 4) as usize);

        for row in y..(y + height as i32) {
            if row < 0 || row >= surface.height as i32 {
                continue;
            }

            for col in x..(x + width as i32) {
                if col < 0 || col >= surface.width as i32 {
                    continue;
                }

                let pixel_offset = ((row as u32 * surface.width + col as u32) * 4) as usize;
                if pixel_offset + 4 <= surface.data.len() {
                    region_data.extend_from_slice(&surface.data[pixel_offset..pixel_offset + 4]);
                }
            }
        }

        // Create ImageData for the region
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&region_data),
            width,
            height,
        )
        .map_err(|_| SpiceError::Protocol("Failed to create region ImageData".to_string()))?;

        // Draw the region to canvas
        display
            .context
            .put_image_data(&image_data, x as f64, y as f64)
            .map_err(|_| SpiceError::Protocol("Failed to put region image data".to_string()))?;

        Ok(())
    }

    /// Removes a canvas
    pub fn remove_canvas(&mut self, surface_id: u32) -> Result<()> {
        if let Some(display) = self.canvases.remove(&surface_id) {
            // Remove from DOM if attached
            if let Some(parent) = display.canvas.parent_element() {
                parent.remove_child(&display.canvas).map_err(|_| {
                    SpiceError::Protocol("Failed to remove canvas from DOM".to_string())
                })?;
            }
        }
        Ok(())
    }

    /// Gets all canvas elements
    pub fn get_all_canvases(&self) -> Vec<&HtmlCanvasElement> {
        self.canvases.values().map(|d| &d.canvas).collect()
    }

    /// Arranges canvases based on monitor configuration
    pub fn arrange_canvases(&self, monitors: &[crate::protocol::SpiceHead]) -> Result<()> {
        for monitor in monitors {
            if let Some(display) = self.canvases.get(&monitor.surface_id) {
                let style = display.canvas.style();
                style.set_property("position", "absolute").map_err(|_| {
                    SpiceError::Protocol("Failed to set canvas position".to_string())
                })?;
                style
                    .set_property("left", &format!("{}px", monitor.x))
                    .map_err(|_| SpiceError::Protocol("Failed to set canvas left".to_string()))?;
                style
                    .set_property("top", &format!("{}px", monitor.y))
                    .map_err(|_| SpiceError::Protocol("Failed to set canvas top".to_string()))?;
            }
        }
        Ok(())
    }
}
