//! WebAssembly video renderer implementation

use crate::channels::display::StreamInfo;
use crate::error::{Result, SpiceError};
use js_sys::Uint8Array;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlVideoElement, MediaSource, SourceBuffer, Url};

/// Manages video rendering for WebAssembly environment
pub struct WasmVideoRenderer {
    video_elements: HashMap<u32, VideoStream>,
    media_sources: HashMap<u32, MediaSource>,
}

struct VideoStream {
    video_element: HtmlVideoElement,
    source_buffer: Option<SourceBuffer>,
    stream_id: u32,
    codec_type: u8,
}

impl WasmVideoRenderer {
    /// Creates a new WebAssembly video renderer
    pub fn new() -> Self {
        Self {
            video_elements: HashMap::new(),
            media_sources: HashMap::new(),
        }
    }

    /// Creates a video element for a stream
    pub fn create_video_stream(&mut self, stream_info: &StreamInfo) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| SpiceError::Protocol("No window object available".to_string()))?;

        let document = window
            .document()
            .ok_or_else(|| SpiceError::Protocol("No document object available".to_string()))?;

        // Create video element
        let video = document
            .create_element("video")
            .map_err(|_| SpiceError::Protocol("Failed to create video element".to_string()))?
            .dyn_into::<HtmlVideoElement>()
            .map_err(|_| SpiceError::Protocol("Failed to cast to HtmlVideoElement".to_string()))?;

        video.set_width(stream_info.width);
        video.set_height(stream_info.height);
        video.set_id(&format!("spice-video-{}", stream_info.id));
        video.set_autoplay(true);
        video.set_muted(true); // Mute to allow autoplay

        // Create MediaSource for streaming
        let media_source = MediaSource::new()
            .map_err(|_| SpiceError::Protocol("Failed to create MediaSource".to_string()))?;

        let url = Url::create_object_url_with_source(&media_source)
            .map_err(|_| SpiceError::Protocol("Failed to create object URL".to_string()))?;

        video.set_src(&url);

        // Store references
        self.media_sources
            .insert(stream_info.id, media_source.clone());
        self.video_elements.insert(
            stream_info.id,
            VideoStream {
                video_element: video,
                source_buffer: None,
                stream_id: stream_info.id,
                codec_type: stream_info.codec_type,
            },
        );

        // Set up source buffer when ready
        let stream_id = stream_info.id;
        let codec_type = stream_info.codec_type;
        let renderer = self as *mut WasmVideoRenderer;

        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| unsafe {
            if let Some(renderer) = renderer.as_mut() {
                let _ = renderer.setup_source_buffer(stream_id, codec_type);
            }
        }) as Box<dyn FnMut(_)>);

        media_source
            .add_event_listener_with_callback("sourceopen", closure.as_ref().unchecked_ref())
            .map_err(|_| SpiceError::Protocol("Failed to add sourceopen listener".to_string()))?;

        closure.forget(); // Keep closure alive

        Ok(())
    }

    /// Sets up the source buffer for a stream
    fn setup_source_buffer(&mut self, stream_id: u32, codec_type: u8) -> Result<()> {
        let media_source = self
            .media_sources
            .get(&stream_id)
            .ok_or_else(|| SpiceError::Protocol("MediaSource not found".to_string()))?;

        // Determine MIME type based on codec
        let mime_type = match codec_type {
            1 => "video/jpeg", // MJPEG
            2 => "video/h264", // H.264
            3 => "video/vp8",  // VP8
            4 => "video/h265", // H.265
            _ => {
                return Err(SpiceError::Protocol(format!(
                    "Unsupported codec type: {}",
                    codec_type
                )))
            }
        };

        // Check if codec is supported
        if !MediaSource::is_type_supported(mime_type) {
            return Err(SpiceError::Protocol(format!(
                "Codec not supported: {}",
                mime_type
            )));
        }

        let source_buffer = media_source
            .add_source_buffer(mime_type)
            .map_err(|_| SpiceError::Protocol("Failed to add source buffer".to_string()))?;

        if let Some(stream) = self.video_elements.get_mut(&stream_id) {
            stream.source_buffer = Some(source_buffer);
        }

        Ok(())
    }

    /// Appends video data to a stream
    pub fn append_video_data(&mut self, stream_id: u32, data: &[u8]) -> Result<()> {
        let stream = self
            .video_elements
            .get(&stream_id)
            .ok_or_else(|| SpiceError::Protocol(format!("Video stream {} not found", stream_id)))?;

        if let Some(source_buffer) = &stream.source_buffer {
            if source_buffer.updating() {
                // Buffer is busy, queue the data
                return Ok(());
            }

            // Create a JavaScript ArrayBuffer and view it as Uint8Array
            let array = Uint8Array::new_with_length(data.len() as u32);
            array.copy_from(data);

            // Use append_buffer with ArrayBuffer instead
            source_buffer
                .append_buffer_with_array_buffer(&array.buffer())
                .map_err(|_| SpiceError::Protocol("Failed to append buffer".to_string()))?;
        }

        Ok(())
    }

    /// Gets the video element for a stream
    pub fn get_video_element(&self, stream_id: u32) -> Option<&HtmlVideoElement> {
        self.video_elements
            .get(&stream_id)
            .map(|s| &s.video_element)
    }

    /// Removes a video stream
    pub fn remove_video_stream(&mut self, stream_id: u32) -> Result<()> {
        if let Some(stream) = self.video_elements.remove(&stream_id) {
            // Remove from DOM if attached
            if let Some(parent) = stream.video_element.parent_element() {
                parent.remove_child(&stream.video_element).map_err(|_| {
                    SpiceError::Protocol("Failed to remove video from DOM".to_string())
                })?;
            }

            // Revoke object URL
            let src = stream.video_element.src();
            if !src.is_empty() {
                Url::revoke_object_url(&src)
                    .map_err(|_| SpiceError::Protocol("Failed to revoke object URL".to_string()))?;
            }
        }

        self.media_sources.remove(&stream_id);
        Ok(())
    }

    /// Positions a video element on the page
    pub fn position_video(&self, stream_id: u32, x: i32, y: i32) -> Result<()> {
        if let Some(stream) = self.video_elements.get(&stream_id) {
            let style = stream.video_element.style();
            style
                .set_property("position", "absolute")
                .map_err(|_| SpiceError::Protocol("Failed to set video position".to_string()))?;
            style
                .set_property("left", &format!("{}px", x))
                .map_err(|_| SpiceError::Protocol("Failed to set video left".to_string()))?;
            style
                .set_property("top", &format!("{}px", y))
                .map_err(|_| SpiceError::Protocol("Failed to set video top".to_string()))?;
        }
        Ok(())
    }
}

/// Performance optimization for WebAssembly
pub struct WasmPerformanceOptimizer {
    frame_skip_threshold: u32,
    current_frame_count: u32,
    last_frame_time: f64,
}

impl WasmPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            frame_skip_threshold: 2,
            current_frame_count: 0,
            last_frame_time: 0.0,
        }
    }

    /// Determines if a frame should be skipped for performance
    pub fn should_skip_frame(&mut self) -> bool {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        let current_time = performance.now();

        let time_delta = current_time - self.last_frame_time;
        self.last_frame_time = current_time;

        // Skip frames if we're falling behind (< 30 FPS)
        if time_delta > 33.0 {
            self.current_frame_count += 1;
            if self.current_frame_count % self.frame_skip_threshold == 0 {
                return true;
            }
        } else {
            self.current_frame_count = 0;
        }

        false
    }

    /// Adjusts quality based on performance
    pub fn get_quality_level(&self) -> u8 {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();

        // Check memory usage if available
        if let Ok(memory) = js_sys::Reflect::get(&performance, &"memory".into()) {
            if let Ok(used_js_heap) = js_sys::Reflect::get(&memory, &"usedJSHeapSize".into()) {
                if let Some(heap_size) = used_js_heap.as_f64() {
                    // Reduce quality if memory usage is high (> 100MB)
                    if heap_size > 100_000_000.0 {
                        return 50; // Low quality
                    }
                }
            }
        }

        100 // Full quality
    }
}
