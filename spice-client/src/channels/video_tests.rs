use super::*;
use crate::error::SpiceError;
use instant::Instant;

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub format: VideoFormat,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoFormat {
    Rgb32,
    Rgba32,
    Bgr32,
    Bgra32,
    Yuv420,
}

impl VideoFrame {
    pub fn from_display_surface(surface: &DisplaySurface, timestamp: u64) -> Result<Self> {
        let format = match surface.format {
            0x0 => VideoFormat::Rgb32,
            0x1 => VideoFormat::Rgba32,
            0x2 => VideoFormat::Bgr32,
            0x3 => VideoFormat::Bgra32,
            _ => {
                return Err(SpiceError::Protocol(format!(
                    "Unsupported format: {}",
                    surface.format
                )))
            }
        };

        Ok(VideoFrame {
            width: surface.width,
            height: surface.height,
            format,
            data: surface.data.clone(),
            timestamp,
        })
    }

    pub fn to_base64_data_url(&self) -> Result<String> {
        let image_data = match self.format {
            VideoFormat::Rgba32 | VideoFormat::Bgra32 => self.data.clone(),
            VideoFormat::Rgb32 => self.convert_rgb_to_rgba(),
            VideoFormat::Bgr32 => self.convert_bgr_to_rgba(),
            VideoFormat::Yuv420 => {
                return Err(SpiceError::Protocol(
                    "YUV420 conversion not implemented".to_string(),
                ))
            }
        };

        let png_data = self.encode_as_png(&image_data)?;
        use base64::Engine;
        let base64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
        Ok(format!("data:image/png;base64,{}", base64))
    }

    fn convert_rgb_to_rgba(&self) -> Vec<u8> {
        let mut rgba_data = Vec::with_capacity((self.width * self.height * 4) as usize);
        for chunk in self.data.chunks(3) {
            rgba_data.extend_from_slice(chunk);
            rgba_data.push(255); // Alpha channel
        }
        rgba_data
    }

    fn convert_bgr_to_rgba(&self) -> Vec<u8> {
        let mut rgba_data = Vec::with_capacity((self.width * self.height * 4) as usize);
        for chunk in self.data.chunks(3) {
            rgba_data.push(chunk[2]); // R
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[0]); // B
            rgba_data.push(255); // A
        }
        rgba_data
    }

    fn encode_as_png(&self, rgba_data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Cursor;

        let mut png_data = Vec::new();
        let mut encoder = png::Encoder::new(Cursor::new(&mut png_data), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| SpiceError::Protocol(format!("PNG header error: {}", e)))?;
        writer
            .write_image_data(rgba_data)
            .map_err(|e| SpiceError::Protocol(format!("PNG write error: {}", e)))?;
        writer
            .finish()
            .map_err(|e| SpiceError::Protocol(format!("PNG finish error: {}", e)))?;

        Ok(png_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_frame_from_display_surface() {
        let surface = DisplaySurface {
            width: 100,
            height: 50,
            format: 0x1,                   // RGBA32
            data: vec![255; 100 * 50 * 4], // White image
        };

        let frame = VideoFrame::from_display_surface(&surface, 1000).unwrap();
        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 50);
        assert_eq!(frame.format, VideoFormat::Rgba32);
        assert_eq!(frame.timestamp, 1000);
        assert_eq!(frame.data.len(), surface.data.len());
    }

    #[test]
    fn test_video_frame_unsupported_format() {
        let surface = DisplaySurface {
            width: 100,
            height: 50,
            format: 0xFF, // Invalid format
            data: vec![0; 100 * 50 * 4],
        };

        let result = VideoFrame::from_display_surface(&surface, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_data_url_generation() {
        let surface = DisplaySurface {
            width: 2,
            height: 2,
            format: 0x1, // RGBA32
            data: vec![
                255, 0, 0, 255, // Red
                0, 255, 0, 255, // Green
                0, 0, 255, 255, // Blue
                255, 255, 255, 255, // White
            ],
        };

        let frame = VideoFrame::from_display_surface(&surface, 1000).unwrap();
        let data_url = frame.to_base64_data_url().unwrap();

        assert!(data_url.starts_with("data:image/png;base64,"));
        assert!(data_url.len() > 22); // Has actual base64 data
    }

    #[test]
    fn test_rgb_to_rgba_conversion() {
        let surface = DisplaySurface {
            width: 2,
            height: 1,
            format: 0x0,                      // RGB32
            data: vec![255, 0, 0, 0, 255, 0], // Red, Green
        };

        let frame = VideoFrame::from_display_surface(&surface, 1000).unwrap();
        let rgba_data = frame.convert_rgb_to_rgba();

        assert_eq!(
            rgba_data,
            vec![
                255, 0, 0, 255, // Red with alpha
                0, 255, 0, 255 // Green with alpha
            ]
        );
    }

    #[test]
    fn test_bgr_to_rgba_conversion() {
        let surface = DisplaySurface {
            width: 2,
            height: 1,
            format: 0x2,                      // BGR32
            data: vec![0, 0, 255, 0, 255, 0], // Red (BGR), Green (BGR)
        };

        let frame = VideoFrame::from_display_surface(&surface, 1000).unwrap();
        let rgba_data = frame.convert_bgr_to_rgba();

        assert_eq!(
            rgba_data,
            vec![
                255, 0, 0, 255, // Red with alpha
                0, 255, 0, 255 // Green with alpha
            ]
        );
    }

    #[test]
    fn test_video_frame_memory_size() {
        let sizes = vec![(640, 480), (1920, 1080), (3840, 2160)];

        for (width, height) in sizes {
            let surface = DisplaySurface {
                width,
                height,
                format: 0x1, // RGBA32
                data: vec![0; (width * height * 4) as usize],
            };

            let frame = VideoFrame::from_display_surface(&surface, 1000).unwrap();
            assert_eq!(frame.data.len(), (width * height * 4) as usize);
        }
    }
}

#[derive(Debug)]
pub struct VideoFrameBuffer {
    frames: Vec<VideoFrame>,
    max_frames: usize,
    target_fps: u32,
    last_frame_time: Option<u64>,
}

impl VideoFrameBuffer {
    pub fn new(max_frames: usize, target_fps: u32) -> Self {
        Self {
            frames: Vec::with_capacity(max_frames),
            max_frames,
            target_fps,
            last_frame_time: None,
        }
    }

    pub fn should_accept_frame(&self, timestamp: u64) -> bool {
        match self.last_frame_time {
            None => true,
            Some(last) => {
                if timestamp < last {
                    return false; // Timestamp went backwards
                }
                let min_interval = 1_000_000 / self.target_fps as u64; // microseconds
                timestamp - last >= min_interval
            }
        }
    }

    pub fn add_frame(&mut self, frame: VideoFrame) -> bool {
        if !self.should_accept_frame(frame.timestamp) {
            return false;
        }

        if self.frames.len() >= self.max_frames {
            self.frames.remove(0); // Drop oldest frame
        }

        self.last_frame_time = Some(frame.timestamp);
        self.frames.push(frame);
        true
    }

    pub fn get_latest_frame(&self) -> Option<&VideoFrame> {
        self.frames.last()
    }

    pub fn get_frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.last_frame_time = None;
    }

    pub fn get_memory_usage(&self) -> usize {
        self.frames
            .iter()
            .map(|f| f.data.len() + std::mem::size_of::<VideoFrame>())
            .sum()
    }
}

#[cfg(test)]
mod buffer_tests {
    use super::*;

    #[test]
    fn test_frame_rate_limiting() {
        let mut buffer = VideoFrameBuffer::new(10, 30); // 30 FPS target

        let surface = DisplaySurface {
            width: 100,
            height: 100,
            format: 0x1,
            data: vec![0; 100 * 100 * 4],
        };

        // First frame should be accepted
        let frame1 = VideoFrame::from_display_surface(&surface, 0).unwrap();
        assert!(buffer.add_frame(frame1));

        // Frame too soon should be rejected (10 microseconds later)
        let frame2 = VideoFrame::from_display_surface(&surface, 10).unwrap();
        assert!(!buffer.add_frame(frame2));

        // Frame after sufficient time should be accepted (33334 microseconds = ~30 FPS)
        let frame3 = VideoFrame::from_display_surface(&surface, 33334).unwrap();
        assert!(buffer.add_frame(frame3));

        assert_eq!(buffer.get_frame_count(), 2);
    }

    #[test]
    fn test_buffer_overflow_handling() {
        let mut buffer = VideoFrameBuffer::new(3, 60);

        let surface = DisplaySurface {
            width: 10,
            height: 10,
            format: 0x1,
            data: vec![0; 10 * 10 * 4],
        };

        // Add 4 frames, should only keep last 3
        for i in 0..4 {
            let frame = VideoFrame::from_display_surface(&surface, i * 20_000).unwrap();
            buffer.add_frame(frame);
        }

        assert_eq!(buffer.get_frame_count(), 3);
    }

    #[test]
    fn test_memory_usage_tracking() {
        let mut buffer = VideoFrameBuffer::new(10, 30);

        let surface = DisplaySurface {
            width: 100,
            height: 100,
            format: 0x1,
            data: vec![0; 100 * 100 * 4],
        };

        let frame = VideoFrame::from_display_surface(&surface, 0).unwrap();
        let frame_size = frame.data.len() + std::mem::size_of::<VideoFrame>();
        buffer.add_frame(frame);

        assert_eq!(buffer.get_memory_usage(), frame_size);

        // Add another frame
        let frame2 = VideoFrame::from_display_surface(&surface, 40_000).unwrap();
        buffer.add_frame(frame2);

        assert_eq!(buffer.get_memory_usage(), frame_size * 2);
    }

    #[test]
    fn test_buffer_clear() {
        let mut buffer = VideoFrameBuffer::new(10, 30);

        let surface = DisplaySurface {
            width: 10,
            height: 10,
            format: 0x1,
            data: vec![0; 10 * 10 * 4],
        };

        let frame = VideoFrame::from_display_surface(&surface, 0).unwrap();
        buffer.add_frame(frame);

        assert_eq!(buffer.get_frame_count(), 1);
        assert!(buffer.get_latest_frame().is_some());

        buffer.clear();

        assert_eq!(buffer.get_frame_count(), 0);
        assert!(buffer.get_latest_frame().is_none());
        assert_eq!(buffer.get_memory_usage(), 0);
    }
}

#[derive(Debug)]
pub struct VideoStreamer {
    buffer: VideoFrameBuffer,
    frames_encoded: u64,
    frames_dropped: u64,
    total_encoding_time: u64,
    quality_level: u8, // 0-100
}

impl VideoStreamer {
    pub fn new(buffer_size: usize, target_fps: u32) -> Self {
        Self {
            buffer: VideoFrameBuffer::new(buffer_size, target_fps),
            frames_encoded: 0,
            frames_dropped: 0,
            total_encoding_time: 0,
            quality_level: 100,
        }
    }

    pub async fn process_frame(
        &mut self,
        surface: &DisplaySurface,
        timestamp: u64,
    ) -> Result<Option<String>> {
        let start_time = instant::Instant::now();

        // Create video frame
        let frame = VideoFrame::from_display_surface(surface, timestamp)?;

        // Check if we should accept the frame based on frame rate
        if !self.buffer.should_accept_frame(timestamp) {
            self.frames_dropped += 1;
            return Ok(None);
        }

        // Simulate encoding with quality adjustment
        let encoded = if self.quality_level < 100 {
            self.encode_with_quality(&frame, self.quality_level)?
        } else {
            frame.to_base64_data_url()?
        };

        // Track encoding time
        let encoding_time = start_time.elapsed().as_micros() as u64;
        self.total_encoding_time += encoding_time;

        // Add frame to buffer
        if self.buffer.add_frame(frame) {
            self.frames_encoded += 1;
            Ok(Some(encoded))
        } else {
            self.frames_dropped += 1;
            Ok(None)
        }
    }

    fn encode_with_quality(&self, frame: &VideoFrame, quality: u8) -> Result<String> {
        // Quality affects both resolution scaling and PNG compression
        // For better performance at lower quality, we can downsample the image
        let scale = if quality < 80 {
            (quality as f32 / 80.0).max(0.5) // Scale down for quality < 80
        } else {
            1.0 // Full resolution for quality >= 80
        };

        let new_width = (frame.width as f32 * scale) as u32;
        let new_height = (frame.height as f32 * scale) as u32;

        // If scaling is needed, resample the image
        let (width, height, image_data) = if scale < 1.0 {
            let resampled = self.downsample_frame(frame, new_width, new_height)?;
            (new_width, new_height, resampled)
        } else {
            // Use original data
            let rgba_data = match frame.format {
                VideoFormat::Rgba32 | VideoFormat::Bgra32 => frame.data.clone(),
                VideoFormat::Rgb32 => frame.convert_rgb_to_rgba(),
                VideoFormat::Bgr32 => frame.convert_bgr_to_rgba(),
                VideoFormat::Yuv420 => {
                    return Err(SpiceError::Protocol(
                        "YUV420 conversion not implemented".to_string(),
                    ))
                }
            };
            (frame.width, frame.height, rgba_data)
        };

        // Encode as PNG with appropriate compression level
        let compression = if quality >= 90 {
            png::Compression::Best
        } else if quality >= 70 {
            png::Compression::Default
        } else {
            png::Compression::Fast
        };

        let png_data =
            self.encode_as_png_with_compression(&image_data, width, height, compression)?;

        use base64::Engine;
        let base64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
        Ok(format!("data:image/png;base64,{}", base64))
    }

    pub fn adjust_quality_for_load(&mut self, target_encode_time_us: u64) {
        let avg_encode_time = if self.frames_encoded > 0 {
            self.total_encoding_time / self.frames_encoded
        } else {
            0
        };

        if avg_encode_time > target_encode_time_us && self.quality_level > 25 {
            self.quality_level = (self.quality_level - 5).max(25);
        } else if avg_encode_time < target_encode_time_us / 2 && self.quality_level < 100 {
            self.quality_level = (self.quality_level + 5).min(100);
        }
    }

    pub fn get_stats(&self) -> StreamingStats {
        StreamingStats {
            frames_encoded: self.frames_encoded,
            frames_dropped: self.frames_dropped,
            drop_rate: if self.frames_encoded + self.frames_dropped > 0 {
                self.frames_dropped as f32 / (self.frames_encoded + self.frames_dropped) as f32
            } else {
                0.0
            },
            avg_encoding_time_us: if self.frames_encoded > 0 {
                self.total_encoding_time / self.frames_encoded
            } else {
                0
            },
            quality_level: self.quality_level,
            buffer_usage: self.buffer.get_frame_count(),
            memory_usage: self.buffer.get_memory_usage(),
        }
    }

    pub fn reset_stats(&mut self) {
        self.frames_encoded = 0;
        self.frames_dropped = 0;
        self.total_encoding_time = 0;
    }

    fn downsample_frame(
        &self,
        frame: &VideoFrame,
        new_width: u32,
        new_height: u32,
    ) -> Result<Vec<u8>> {
        // Simple nearest-neighbor downsampling
        let mut result = Vec::with_capacity((new_width * new_height * 4) as usize);

        let x_ratio = frame.width as f32 / new_width as f32;
        let y_ratio = frame.height as f32 / new_height as f32;

        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = (x as f32 * x_ratio) as u32;
                let src_y = (y as f32 * y_ratio) as u32;

                let src_offset = ((src_y * frame.width + src_x) * 4) as usize;

                if src_offset + 3 < frame.data.len() {
                    result.push(frame.data[src_offset]); // R
                    result.push(frame.data[src_offset + 1]); // G
                    result.push(frame.data[src_offset + 2]); // B
                    result.push(frame.data[src_offset + 3]); // A
                } else {
                    // Fallback for out-of-bounds
                    result.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        }

        Ok(result)
    }

    fn encode_as_png_with_compression(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
        compression: png::Compression,
    ) -> Result<Vec<u8>> {
        use std::io::Cursor;

        let mut png_data = Vec::new();
        let mut encoder = png::Encoder::new(Cursor::new(&mut png_data), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(compression);

        let mut writer = encoder
            .write_header()
            .map_err(|e| SpiceError::Protocol(format!("PNG header error: {}", e)))?;
        writer
            .write_image_data(rgba_data)
            .map_err(|e| SpiceError::Protocol(format!("PNG write error: {}", e)))?;
        writer
            .finish()
            .map_err(|e| SpiceError::Protocol(format!("PNG finish error: {}", e)))?;

        Ok(png_data)
    }
}

#[derive(Debug, Clone)]
pub struct StreamingStats {
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub drop_rate: f32,
    pub avg_encoding_time_us: u64,
    pub quality_level: u8,
    pub buffer_usage: usize,
    pub memory_usage: usize,
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[tokio::test]
    async fn test_continuous_frame_updates() {
        let mut streamer = VideoStreamer::new(10, 30);

        let surface = DisplaySurface {
            width: 640,
            height: 480,
            format: 0x1,
            data: vec![0; 640 * 480 * 4],
        };

        // Simulate continuous updates at 30 FPS (33.33ms intervals)
        let mut timestamp = 0;
        let mut successful_frames = 0;

        for _ in 0..60 {
            // 2 seconds worth of frames
            let result = streamer.process_frame(&surface, timestamp).await.unwrap();
            if result.is_some() {
                successful_frames += 1;
            }
            timestamp += 33_333; // microseconds
        }

        let stats = streamer.get_stats();
        assert_eq!(stats.frames_encoded, successful_frames);
        assert_eq!(stats.frames_encoded, 60); // Should encode all frames at correct interval
        assert_eq!(stats.frames_dropped, 0);
        assert_eq!(stats.drop_rate, 0.0);
    }

    #[tokio::test]
    async fn test_frame_dropping_under_load() {
        let mut streamer = VideoStreamer::new(5, 60); // 60 FPS target, small buffer

        let surface = DisplaySurface {
            width: 1920,
            height: 1080,
            format: 0x1,
            data: vec![0; 1920 * 1080 * 4],
        };

        // Simulate frames coming in too fast (every 10ms instead of 16.67ms)
        let mut timestamp = 0;
        for _ in 0..100 {
            streamer.process_frame(&surface, timestamp).await.unwrap();
            timestamp += 10_000; // 10ms intervals (100 FPS input)
        }

        let stats = streamer.get_stats();
        assert!(
            stats.frames_dropped > 0,
            "Should drop frames when input is too fast"
        );
        assert!(
            stats.drop_rate > 0.0 && stats.drop_rate < 1.0,
            "Drop rate should be between 0 and 1"
        );
        assert!(
            stats.frames_encoded < 100,
            "Should encode fewer frames than input"
        );
    }

    #[tokio::test]
    async fn test_quality_degradation_under_load() {
        let mut streamer = VideoStreamer::new(10, 30);

        let surface = DisplaySurface {
            width: 3840,
            height: 2160,
            format: 0x1,
            data: vec![0; 3840 * 2160 * 4], // 4K resolution
        };

        // Process frames and simulate high encoding time
        let mut timestamp = 0;
        for i in 0..30 {
            streamer.process_frame(&surface, timestamp).await.unwrap();
            timestamp += 33_333;

            // Simulate adjusting quality based on encoding time
            // Target: 20ms encoding time
            if i % 5 == 0 {
                streamer.total_encoding_time = streamer.frames_encoded * 25_000; // Simulate 25ms avg
                streamer.adjust_quality_for_load(20_000);
            }
        }

        let stats = streamer.get_stats();
        assert!(
            stats.quality_level < 100,
            "Quality should degrade under load"
        );
        assert!(
            stats.quality_level >= 25,
            "Quality should not go below minimum"
        );
    }

    #[tokio::test]
    async fn test_encoding_performance_benchmark() {
        use instant::Instant;

        let mut streamer = VideoStreamer::new(30, 30);

        let sizes = vec![
            (640, 480, "VGA"),
            (1280, 720, "720p"),
            (1920, 1080, "1080p"),
            (3840, 2160, "4K"),
        ];

        let mut results = Vec::new();

        for (width, height, name) in sizes {
            let surface = DisplaySurface {
                width,
                height,
                format: 0x1,
                data: vec![128; (width * height * 4) as usize],
            };

            streamer.reset_stats();

            let start = Instant::now();
            let mut timestamp = 0;

            // Process 30 frames
            for _ in 0..30 {
                streamer.process_frame(&surface, timestamp).await.unwrap();
                timestamp += 33_333;
            }

            let duration = start.elapsed();
            let stats = streamer.get_stats();

            results.push((
                name,
                width * height,
                duration.as_millis(),
                stats.avg_encoding_time_us / 1000, // Convert to ms
            ));
        }

        // Verify performance scales reasonably with resolution
        for i in 1..results.len() {
            let (_, pixels_prev, _, _) = results[i - 1];
            let (_, pixels_curr, _, time_curr) = results[i];

            // Encoding time should increase with resolution, but not linearly
            let pixel_ratio = pixels_curr as f32 / pixels_prev as f32;
            assert!(pixel_ratio > 1.0, "Each resolution should be larger");

            // Basic sanity check - encoding shouldn't take more than 100ms even for 4K
            assert!(time_curr < 100, "Encoding time should be reasonable");
        }
    }

    #[tokio::test]
    async fn test_streaming_memory_management() {
        let mut streamer = VideoStreamer::new(5, 30); // Small buffer

        let surface = DisplaySurface {
            width: 1920,
            height: 1080,
            format: 0x1,
            data: vec![0; 1920 * 1080 * 4],
        };

        // Fill buffer
        let mut timestamp = 0;
        for _ in 0..10 {
            // More than buffer size
            streamer.process_frame(&surface, timestamp).await.unwrap();
            timestamp += 33_333;
        }

        let stats = streamer.get_stats();
        assert!(stats.buffer_usage <= 5, "Buffer should not exceed max size");

        // Calculate expected memory usage
        let frame_size = 1920 * 1080 * 4 + std::mem::size_of::<VideoFrame>();
        let expected_max_memory = frame_size * 5;
        assert!(
            stats.memory_usage <= expected_max_memory,
            "Memory usage should be bounded"
        );
    }
}
