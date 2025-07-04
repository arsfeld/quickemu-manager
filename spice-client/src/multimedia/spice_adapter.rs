use super::{
    display::{CursorData, Display, DisplayMode, PixelFormat},
    input::{InputEvent, KeyboardEvent, MouseButton, MouseEvent},
    Result,
};
use crate::channels::cursor::CursorShape;
use crate::channels::display::DisplaySurface;
use crate::channels::{
    InputEvent as SpiceInputEvent, KeyCode as SpiceKeyCode, MouseButton as SpiceMouseButton,
};
use crate::SpiceClientShared;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Adapter that connects SPICE display channel to multimedia display backend
pub struct SpiceDisplayAdapter {
    client: SpiceClientShared,
    backend_display: Arc<Mutex<Box<dyn Display + Send>>>,
    channel_id: u8,
    current_dimensions: Arc<Mutex<(u32, u32)>>,
    last_frame_hash: Arc<Mutex<Option<u64>>>,
}

impl SpiceDisplayAdapter {
    pub fn new(
        client: SpiceClientShared,
        backend_display: Box<dyn Display + Send>,
        channel_id: u8,
    ) -> Self {
        Self {
            client,
            backend_display: Arc::new(Mutex::new(backend_display)),
            channel_id,
            current_dimensions: Arc::new(Mutex::new((1024, 768))), // Default dimensions
            last_frame_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Updates the display with the latest frame from SPICE
    pub async fn update_display(&self) -> Result<()> {
        eprintln!("SpiceDisplayAdapter: update_display called");
        if let Some(surface) = self.client.get_display_surface(self.channel_id).await {
            eprintln!(
                "SpiceDisplayAdapter: Got surface {}x{} with {} bytes",
                surface.width,
                surface.height,
                surface.data.len()
            );

            // Calculate a simple hash to detect changes
            let mut hash = 0u64;
            for (_i, &byte) in surface.data.iter().enumerate().step_by(1024) {
                hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
            }

            let mut last_hash = self.last_frame_hash.lock().await;
            let changed = match *last_hash {
                Some(prev_hash) => prev_hash != hash,
                None => true,
            };

            if changed {
                eprintln!(
                    "SpiceDisplayAdapter: Display surface changed (hash: {}), updating display",
                    hash
                );
                debug!("Display surface changed, updating display");
                *last_hash = Some(hash);

                // Check if dimensions changed
                let mut current_dims = self.current_dimensions.lock().await;
                if (surface.width, surface.height) != *current_dims {
                    debug!(
                        "Display dimensions changed from {:?} to ({}, {})",
                        current_dims, surface.width, surface.height
                    );
                    let mut display = self.backend_display.lock().await;
                    display.resize(surface.width, surface.height)?;
                    *current_dims = (surface.width, surface.height);
                }

                // Convert SPICE pixel format to our PixelFormat
                let pixel_format = match surface.format {
                    1 => PixelFormat::Rgba8888, // SPICE_SURFACE_FMT_32_xRGB
                    8 => PixelFormat::Rgba8888, // SPICE_SURFACE_FMT_32_ARGB
                    _ => {
                        warn!(
                            "Unknown SPICE surface format: {}, assuming RGBA",
                            surface.format
                        );
                        PixelFormat::Rgba8888
                    }
                };

                // Present the frame
                let mut display = self.backend_display.lock().await;
                display.present_frame(&surface.data, pixel_format)?;
            }
        } else {
            eprintln!("SpiceDisplayAdapter: No surface available from SPICE client");
        }

        Ok(())
    }

    /// Gets the backend display for direct access
    pub fn get_backend_display(&self) -> Arc<Mutex<Box<dyn Display + Send>>> {
        self.backend_display.clone()
    }

    /// Updates the cursor with data from the cursor channel
    pub async fn update_cursor(&self, cursor_shape: Option<&CursorShape>) -> Result<()> {
        let mut display = self.backend_display.lock().await;

        if let Some(shape) = cursor_shape {
            let cursor_data = CursorData {
                width: shape.width as u32,
                height: shape.height as u32,
                hotspot_x: shape.hot_spot_x as u32,
                hotspot_y: shape.hot_spot_y as u32,
                data: shape.data.clone(),
                format: PixelFormat::Rgba8888, // SPICE cursors are RGBA
            };

            debug!(
                "Updating cursor: {}x{}, hotspot: ({}, {})",
                cursor_data.width, cursor_data.height, cursor_data.hotspot_x, cursor_data.hotspot_y
            );

            display.set_cursor(Some(cursor_data))?;
        } else {
            debug!("Hiding cursor");
            display.set_cursor(None)?;
        }

        Ok(())
    }
}

/// Converts SPICE display surface to multimedia pixel format
fn convert_spice_format(spice_format: u32) -> PixelFormat {
    match spice_format {
        1 => PixelFormat::Rgba8888, // SPICE_SURFACE_FMT_32_xRGB
        8 => PixelFormat::Rgba8888, // SPICE_SURFACE_FMT_32_ARGB
        16 => PixelFormat::Rgb565,  // SPICE_SURFACE_FMT_16_565
        _ => {
            warn!(
                "Unknown SPICE surface format: {}, defaulting to RGBA",
                spice_format
            );
            PixelFormat::Rgba8888
        }
    }
}

/// Adapter that connects multimedia input events to SPICE inputs channel
pub struct SpiceInputAdapter {
    client: SpiceClientShared,
    channel_id: u8,
}

impl SpiceInputAdapter {
    pub fn new(client: SpiceClientShared, channel_id: u8) -> Self {
        Self { client, channel_id }
    }

    /// Forwards an input event to the SPICE inputs channel
    pub async fn send_event(&self, event: InputEvent) -> Result<()> {
        // TODO: We need to access the inputs channel from the client
        // For now, just log the events
        match event {
            InputEvent::Keyboard(kbd_event) => match kbd_event {
                KeyboardEvent::KeyDown { scancode, .. } => {
                    debug!("Forward key down: scancode {}", scancode);
                    self.client.send_key_down(self.channel_id, scancode).await?;
                }
                KeyboardEvent::KeyUp { scancode, .. } => {
                    debug!("Forward key up: scancode {}", scancode);
                    self.client.send_key_up(self.channel_id, scancode).await?;
                }
            },
            InputEvent::Mouse(mouse_event) => match mouse_event {
                MouseEvent::Motion { x, y, .. } => {
                    debug!("Forward mouse motion: ({}, {})", x, y);
                    self.client
                        .send_mouse_motion(self.channel_id, x as i32, y as i32)
                        .await?;
                }
                MouseEvent::Button {
                    button,
                    pressed,
                    x,
                    y,
                } => {
                    let spice_button = convert_mouse_button(button);
                    debug!(
                        "Forward mouse button {:?}: {} at ({}, {})",
                        button, pressed, x, y
                    );
                    self.client
                        .send_mouse_button(self.channel_id, spice_button, pressed)
                        .await?;
                }
                MouseEvent::Wheel { delta_x, delta_y } => {
                    debug!("Forward mouse wheel: ({}, {})", delta_x, delta_y);
                    self.client
                        .send_mouse_wheel(self.channel_id, delta_x, delta_y)
                        .await?;
                }
            },
        }

        Ok(())
    }
}

fn convert_mouse_button(button: MouseButton) -> SpiceMouseButton {
    match button {
        MouseButton::Left => SpiceMouseButton::Left,
        MouseButton::Middle => SpiceMouseButton::Middle,
        MouseButton::Right => SpiceMouseButton::Right,
        // SPICE protocol only supports 3 buttons, map extra buttons to middle
        MouseButton::X1 | MouseButton::X2 => SpiceMouseButton::Middle,
    }
}
