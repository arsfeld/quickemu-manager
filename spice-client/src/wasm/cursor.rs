//! WebAssembly cursor rendering

use crate::channels::cursor::CursorShape;
use crate::error::{Result, SpiceError};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CssStyleDeclaration, HtmlElement};

/// Manages cursor rendering in WebAssembly environment
pub struct WasmCursorRenderer {
    cursor_element: Option<HtmlElement>,
    container_element: Option<HtmlElement>,
}

impl WasmCursorRenderer {
    pub fn new() -> Self {
        Self {
            cursor_element: None,
            container_element: None,
        }
    }

    /// Sets the container element for cursor rendering
    pub fn set_container(&mut self, container_id: &str) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object".to_string()))?;

        let document = window
            .document()
            .ok_or_else(|| SpiceError::Protocol("No document object".to_string()))?;

        let container = document
            .get_element_by_id(container_id)
            .ok_or_else(|| SpiceError::Protocol(format!("Container '{}' not found", container_id)))?
            .dyn_into::<HtmlElement>()
            .map_err(|_| SpiceError::Protocol("Failed to cast to HtmlElement".to_string()))?;

        self.container_element = Some(container);
        Ok(())
    }

    /// Updates the cursor shape
    pub fn update_cursor(&mut self, cursor: &CursorShape) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object".to_string()))?;

        let document = window
            .document()
            .ok_or_else(|| SpiceError::Protocol("No document object".to_string()))?;

        // Create or get cursor element
        let cursor_elem = if let Some(elem) = &self.cursor_element {
            elem.clone()
        } else {
            let elem = document
                .create_element("div")
                .map_err(|_| SpiceError::Protocol("Failed to create cursor element".to_string()))?
                .dyn_into::<HtmlElement>()
                .map_err(|_| SpiceError::Protocol("Failed to cast to HtmlElement".to_string()))?;

            elem.set_id("spice-cursor");
            elem.set_class_name("spice-cursor");

            // Set initial styles
            let style = elem.style();
            style
                .set_property("position", "absolute")
                .map_err(|_| SpiceError::Protocol("Failed to set cursor position".to_string()))?;
            style
                .set_property("pointer-events", "none")
                .map_err(|_| SpiceError::Protocol("Failed to set pointer-events".to_string()))?;
            style
                .set_property("z-index", "9999")
                .map_err(|_| SpiceError::Protocol("Failed to set z-index".to_string()))?;

            // Append to container if available
            if let Some(container) = &self.container_element {
                container.append_child(&elem).map_err(|_| {
                    SpiceError::Protocol("Failed to append cursor to container".to_string())
                })?;
            }

            self.cursor_element = Some(elem.clone());
            elem
        };

        // Convert cursor data to data URL
        let data_url = self.cursor_to_data_url(cursor)?;

        // Update cursor style
        let style = cursor_elem.style();
        style
            .set_property("width", &format!("{}px", cursor.width))
            .map_err(|_| SpiceError::Protocol("Failed to set cursor width".to_string()))?;
        style
            .set_property("height", &format!("{}px", cursor.height))
            .map_err(|_| SpiceError::Protocol("Failed to set cursor height".to_string()))?;
        style
            .set_property("background-image", &format!("url({})", data_url))
            .map_err(|_| SpiceError::Protocol("Failed to set cursor image".to_string()))?;
        style
            .set_property("background-size", "contain")
            .map_err(|_| SpiceError::Protocol("Failed to set background-size".to_string()))?;

        // Adjust for hotspot
        style
            .set_property("margin-left", &format!("-{}px", cursor.hot_spot_x))
            .map_err(|_| SpiceError::Protocol("Failed to set margin-left".to_string()))?;
        style
            .set_property("margin-top", &format!("-{}px", cursor.hot_spot_y))
            .map_err(|_| SpiceError::Protocol("Failed to set margin-top".to_string()))?;

        Ok(())
    }

    /// Moves the cursor to a specific position
    pub fn move_cursor(&self, x: i32, y: i32) -> Result<()> {
        if let Some(cursor_elem) = &self.cursor_element {
            let style = cursor_elem.style();
            style
                .set_property("left", &format!("{}px", x))
                .map_err(|_| SpiceError::Protocol("Failed to set cursor left".to_string()))?;
            style
                .set_property("top", &format!("{}px", y))
                .map_err(|_| SpiceError::Protocol("Failed to set cursor top".to_string()))?;
        }
        Ok(())
    }

    /// Shows or hides the cursor
    pub fn set_cursor_visible(&self, visible: bool) -> Result<()> {
        if let Some(cursor_elem) = &self.cursor_element {
            let style = cursor_elem.style();
            style
                .set_property("display", if visible { "block" } else { "none" })
                .map_err(|_| SpiceError::Protocol("Failed to set cursor visibility".to_string()))?;
        }
        Ok(())
    }

    /// Converts cursor shape to data URL
    fn cursor_to_data_url(&self, cursor: &CursorShape) -> Result<String> {
        // Create a canvas to render the cursor
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object".to_string()))?;

        let document = window
            .document()
            .ok_or_else(|| SpiceError::Protocol("No document object".to_string()))?;

        let canvas = document
            .create_element("canvas")
            .map_err(|_| SpiceError::Protocol("Failed to create canvas".to_string()))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| SpiceError::Protocol("Failed to cast to HtmlCanvasElement".to_string()))?;

        canvas.set_width(cursor.width as u32);
        canvas.set_height(cursor.height as u32);

        let context = canvas
            .get_context("2d")
            .map_err(|_| SpiceError::Protocol("Failed to get 2D context".to_string()))?
            .ok_or_else(|| SpiceError::Protocol("No 2D context available".to_string()))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .map_err(|_| {
                SpiceError::Protocol("Failed to cast to CanvasRenderingContext2d".to_string())
            })?;

        // Create ImageData from cursor data
        let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&cursor.data),
            cursor.width as u32,
            cursor.height as u32,
        )
        .map_err(|_| SpiceError::Protocol("Failed to create ImageData".to_string()))?;

        context
            .put_image_data(&image_data, 0.0, 0.0)
            .map_err(|_| SpiceError::Protocol("Failed to put image data".to_string()))?;

        // Convert to data URL
        canvas
            .to_data_url()
            .map_err(|_| SpiceError::Protocol("Failed to convert canvas to data URL".to_string()))
    }

    /// Uses CSS cursor for better performance
    pub fn use_css_cursor(&self, cursor: &CursorShape) -> Result<()> {
        let data_url = self.cursor_to_data_url(cursor)?;

        if let Some(container) = &self.container_element {
            let style = container.style();
            let cursor_css = format!(
                "url({}) {} {}, auto",
                data_url, cursor.hot_spot_x, cursor.hot_spot_y
            );

            style
                .set_property("cursor", &cursor_css)
                .map_err(|_| SpiceError::Protocol("Failed to set CSS cursor".to_string()))?;
        }

        Ok(())
    }
}
