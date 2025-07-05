use crate::multimedia::{
    display::{CursorData, Display, DisplayMode, PixelFormat},
    MultimediaError, Result,
};
use gdk_pixbuf;
use gtk4::{cairo, gdk, prelude::*, DrawingArea};
use std::sync::{Arc, Mutex};

pub struct Gtk4Display {
    drawing_area: Option<DrawingArea>,
    surface_data: Arc<Mutex<Option<SurfaceData>>>,
    cursor_data: Arc<Mutex<Option<CursorData>>>,
    dimensions: (u32, u32),
    fullscreen: bool,
    window: Option<gtk4::Window>,
}

struct SurfaceData {
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: PixelFormat,
}

// Safety: GTK4 display is designed to be accessed from the main thread
unsafe impl Send for Gtk4Display {}

impl Gtk4Display {
    pub fn new() -> Result<Self> {
        Ok(Self {
            drawing_area: None,
            surface_data: Arc::new(Mutex::new(None)),
            cursor_data: Arc::new(Mutex::new(None)),
            dimensions: (0, 0),
            fullscreen: false,
            window: None,
        })
    }

    fn setup_drawing_area(&mut self) -> Result<()> {
        let drawing_area = DrawingArea::builder()
            .width_request(self.dimensions.0 as i32)
            .height_request(self.dimensions.1 as i32)
            .build();

        let surface_data = self.surface_data.clone();
        drawing_area.set_draw_func(move |_area, cr, width, height| {
            eprintln!(
                "GTK4 Display: Draw function called, area size: {width}x{height}"
            );

            // Clear background
            cr.set_source_rgb(0.0, 0.0, 0.0);
            let _ = cr.paint();

            // Draw surface data if available
            if let Ok(guard) = surface_data.lock() {
                if let Some(ref data) = *guard {
                    eprintln!(
                        "GTK4 Display: Drawing surface data {}x{}", data.width, data.height
                    );
                    if let Err(e) = draw_surface(cr, data, width, height) {
                        eprintln!("Failed to draw surface: {e}");
                    }
                } else {
                    eprintln!("GTK4 Display: No surface data available");
                }
            } else {
                eprintln!("GTK4 Display: Failed to lock surface data");
            }
        });

        self.drawing_area = Some(drawing_area);
        Ok(())
    }
}

fn draw_surface(
    cr: &cairo::Context,
    surface_data: &SurfaceData,
    width: i32,
    height: i32,
) -> Result<()> {
    let surface = match surface_data.format {
        PixelFormat::Rgba8888 | PixelFormat::Bgra8888 => {
            // Convert to Cairo format if needed
            let mut cairo_data = surface_data.data.clone();

            // Cairo expects ARGB32 in native endian, which is BGRA in little-endian
            if surface_data.format == PixelFormat::Rgba8888 {
                // Swap R and B channels
                for chunk in cairo_data.chunks_mut(4) {
                    chunk.swap(0, 2);
                }
            }

            // Create surface with owned data

            cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                surface_data.width as i32,
                surface_data.height as i32,
            )
            .map_err(|e| MultimediaError::new(format!("Failed to create Cairo surface: {e}")))
            .and_then(|mut surface| {
                // Copy data to the surface
                let mut surface_data_ref = surface.data().map_err(|e| {
                    MultimediaError::new(format!("Failed to get surface data: {e}"))
                })?;

                let data_slice = surface_data_ref.as_mut();
                let copy_len = data_slice.len().min(cairo_data.len());
                data_slice[..copy_len].copy_from_slice(&cairo_data[..copy_len]);

                drop(surface_data_ref); // Release the data lock
                Ok(surface)
            })?
        }
        PixelFormat::Rgb888 | PixelFormat::Bgr888 => {
            // Convert RGB to ARGB
            let mut cairo_data = Vec::with_capacity(surface_data.data.len() * 4 / 3);

            for chunk in surface_data.data.chunks(3) {
                if surface_data.format == PixelFormat::Rgb888 {
                    cairo_data.push(255); // A
                    cairo_data.push(chunk[2]); // B
                    cairo_data.push(chunk[1]); // G
                    cairo_data.push(chunk[0]); // R
                } else {
                    cairo_data.push(255); // A
                    cairo_data.push(chunk[0]); // B
                    cairo_data.push(chunk[1]); // G
                    cairo_data.push(chunk[2]); // R
                }
            }

            cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                surface_data.width as i32,
                surface_data.height as i32,
            )
            .map_err(|e| MultimediaError::new(format!("Failed to create Cairo surface: {e}")))
            .and_then(|mut surface| {
                // Copy data to the surface
                let mut surface_data_ref = surface.data().map_err(|e| {
                    MultimediaError::new(format!("Failed to get surface data: {e}"))
                })?;

                let data_slice = surface_data_ref.as_mut();
                let copy_len = data_slice.len().min(cairo_data.len());
                data_slice[..copy_len].copy_from_slice(&cairo_data[..copy_len]);

                drop(surface_data_ref); // Release the data lock
                Ok(surface)
            })?
        }
        PixelFormat::Rgb565 => {
            // Convert RGB565 to ARGB
            let mut cairo_data = Vec::with_capacity(surface_data.data.len() * 2);

            for chunk in surface_data.data.chunks(2) {
                let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
                let r = ((pixel >> 11) & 0x1F) << 3;
                let g = ((pixel >> 5) & 0x3F) << 2;
                let b = (pixel & 0x1F) << 3;

                cairo_data.push(255); // A
                cairo_data.push(b as u8); // B
                cairo_data.push(g as u8); // G
                cairo_data.push(r as u8); // R
            }

            cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                surface_data.width as i32,
                surface_data.height as i32,
            )
            .map_err(|e| MultimediaError::new(format!("Failed to create Cairo surface: {e}")))
            .and_then(|mut surface| {
                // Copy data to the surface
                let mut surface_data_ref = surface.data().map_err(|e| {
                    MultimediaError::new(format!("Failed to get surface data: {e}"))
                })?;

                let data_slice = surface_data_ref.as_mut();
                let copy_len = data_slice.len().min(cairo_data.len());
                data_slice[..copy_len].copy_from_slice(&cairo_data[..copy_len]);

                drop(surface_data_ref); // Release the data lock
                Ok(surface)
            })?
        }
    };

    // Scale to fit if needed
    let scale_x = width as f64 / surface_data.width as f64;
    let scale_y = height as f64 / surface_data.height as f64;
    let scale = scale_x.min(scale_y);

    if scale != 1.0 {
        cr.scale(scale, scale);
    }

    cr.set_source_surface(&surface, 0.0, 0.0)
        .map_err(|e| MultimediaError::new(format!("Failed to set source surface: {e}")))?;
    cr.paint()
        .map_err(|e| MultimediaError::new(format!("Failed to paint surface: {e}")))?;

    Ok(())
}

impl Display for Gtk4Display {
    fn create_surface(&mut self, mode: DisplayMode) -> Result<()> {
        self.dimensions = (mode.width, mode.height);
        self.fullscreen = mode.fullscreen;

        self.setup_drawing_area()?;

        // Note: Window creation is handled by the application, not here
        // The drawing area will be added to the window by the app

        Ok(())
    }

    fn present_frame(&mut self, data: &[u8], format: PixelFormat) -> Result<()> {
        let (width, height) = self.dimensions;

        // Validate data size
        let expected_size = match format {
            PixelFormat::Rgba8888 | PixelFormat::Bgra8888 => (width * height * 4) as usize,
            PixelFormat::Rgb888 | PixelFormat::Bgr888 => (width * height * 3) as usize,
            PixelFormat::Rgb565 => (width * height * 2) as usize,
        };

        if data.len() != expected_size {
            eprintln!(
                "GTK4 Display: Invalid data size: expected {expected_size} bytes, got {} bytes for {width}x{height} {format:?}",
                data.len()
            );
            return Err(MultimediaError::new(format!(
                "Invalid data size: expected {expected_size} bytes, got {} bytes",
                data.len()
            )));
        }

        eprintln!(
            "GTK4 Display: Presenting frame {width}x{height} format {format:?} with {} bytes",
            data.len()
        );

        // Update surface data
        {
            let mut guard = self
                .surface_data
                .lock()
                .map_err(|e| MultimediaError::new(format!("Failed to lock surface data: {e}")))?;

            *guard = Some(SurfaceData {
                data: data.to_vec(),
                width,
                height,
                format,
            });
        }

        // Queue redraw
        if let Some(ref drawing_area) = self.drawing_area {
            eprintln!("GTK4 Display: Queuing redraw");
            drawing_area.queue_draw();
        } else {
            eprintln!("GTK4 Display: WARNING - No drawing area available!");
        }

        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.dimensions = (width, height);

        if let Some(ref drawing_area) = self.drawing_area {
            drawing_area.set_size_request(width as i32, height as i32);
        }

        Ok(())
    }

    fn set_cursor(&mut self, cursor: Option<CursorData>) -> Result<()> {
        // Update cursor data
        {
            let mut guard = self
                .cursor_data
                .lock()
                .map_err(|e| MultimediaError::new(format!("Failed to lock cursor data: {e}")))?;
            *guard = cursor.clone();
        }

        // Apply cursor to the drawing area if available
        if let Some(ref drawing_area) = self.drawing_area {
            if let Some(ref cursor_data) = cursor {
                // Create a GdkPixbuf from cursor data
                let pixbuf = gdk_pixbuf::Pixbuf::from_mut_slice(
                    cursor_data.data.clone(),
                    gdk_pixbuf::Colorspace::Rgb,
                    true, // has_alpha
                    8,    // bits_per_sample
                    cursor_data.width as i32,
                    cursor_data.height as i32,
                    cursor_data.width as i32 * 4, // rowstride
                );

                // Create a GdkCursor from the pixbuf
                let _display = drawing_area.display();
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                let cursor = gdk::Cursor::from_texture(
                    &texture,
                    cursor_data.hotspot_x as i32,
                    cursor_data.hotspot_y as i32,
                    None,
                );

                drawing_area.set_cursor(Some(&cursor));
            } else {
                // Reset to default cursor
                drawing_area.set_cursor(None::<&gdk::Cursor>);
            }
        }

        Ok(())
    }

    fn set_title(&mut self, title: &str) -> Result<()> {
        if let Some(ref window) = self.window {
            window.set_title(Some(title));
        }
        Ok(())
    }

    fn toggle_fullscreen(&mut self) -> Result<()> {
        if let Some(ref window) = self.window {
            if self.fullscreen {
                window.unfullscreen();
            } else {
                window.fullscreen();
            }
            self.fullscreen = !self.fullscreen;
        }
        Ok(())
    }

    fn get_dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Gtk4Display {
    /// Get the GTK4 DrawingArea widget for integration with the app
    pub fn get_drawing_area(&self) -> Option<&DrawingArea> {
        self.drawing_area.as_ref()
    }

    /// Set the window reference for fullscreen and title operations
    pub fn set_window(&mut self, window: gtk4::Window) {
        self.window = Some(window);
    }
}
