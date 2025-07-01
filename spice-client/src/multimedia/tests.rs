#[cfg(test)]
mod tests {
    use crate::multimedia::{
        display::{Display, DisplayMode, PixelFormat, CursorData},
        audio::{AudioOutput, AudioFormat},
        input::{InputHandler, KeyCode, KeyboardEvent, MouseButton, MouseEvent},
        AudioSpec, MultimediaError, Result,
    };

    #[test]
    fn test_pixel_format_bytes_per_pixel() {
        assert_eq!(PixelFormat::Rgb888.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Rgba8888.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Bgr888.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Bgra8888.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb565.bytes_per_pixel(), 2);
    }

    #[test]
    fn test_audio_format_bytes_per_sample() {
        assert_eq!(AudioFormat::U8.bytes_per_sample(), 1);
        assert_eq!(AudioFormat::S16.bytes_per_sample(), 2);
        assert_eq!(AudioFormat::S32.bytes_per_sample(), 4);
        assert_eq!(AudioFormat::F32.bytes_per_sample(), 4);
    }

    #[test]
    fn test_audio_spec_default() {
        let spec = AudioSpec::default();
        assert_eq!(spec.frequency, 44100);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.samples, 4096);
    }

    #[test]
    fn test_display_mode_default() {
        let mode = DisplayMode::default();
        assert_eq!(mode.width, 1024);
        assert_eq!(mode.height, 768);
        assert!(!mode.fullscreen);
    }

    #[test]
    fn test_multimedia_error() {
        let error = MultimediaError::new("Test error");
        assert_eq!(error.to_string(), "Multimedia error: Test error");
    }

    // Mock implementations for testing
    struct MockDisplay {
        dimensions: (u32, u32),
        fullscreen: bool,
        title: String,
    }

    impl MockDisplay {
        fn new() -> Self {
            Self {
                dimensions: (0, 0),
                fullscreen: false,
                title: String::new(),
            }
        }
    }

    impl Display for MockDisplay {
        fn create_surface(&mut self, mode: DisplayMode) -> Result<()> {
            self.dimensions = (mode.width, mode.height);
            self.fullscreen = mode.fullscreen;
            Ok(())
        }

        fn present_frame(&mut self, _data: &[u8], _format: PixelFormat) -> Result<()> {
            Ok(())
        }

        fn resize(&mut self, width: u32, height: u32) -> Result<()> {
            self.dimensions = (width, height);
            Ok(())
        }

        fn set_cursor(&mut self, _cursor: Option<crate::multimedia::display::CursorData>) -> Result<()> {
            Ok(())
        }

        fn set_title(&mut self, title: &str) -> Result<()> {
            self.title = title.to_string();
            Ok(())
        }

        fn toggle_fullscreen(&mut self) -> Result<()> {
            self.fullscreen = !self.fullscreen;
            Ok(())
        }

        fn get_dimensions(&self) -> (u32, u32) {
            self.dimensions
        }

        fn is_fullscreen(&self) -> bool {
            self.fullscreen
        }
    }

    #[test]
    fn test_mock_display() {
        let mut display = MockDisplay::new();
        
        // Test surface creation
        display.create_surface(DisplayMode {
            width: 1920,
            height: 1080,
            fullscreen: true,
        }).unwrap();
        
        assert_eq!(display.get_dimensions(), (1920, 1080));
        assert!(display.is_fullscreen());
        
        // Test resize
        display.resize(1280, 720).unwrap();
        assert_eq!(display.get_dimensions(), (1280, 720));
        
        // Test title
        display.set_title("Test Title").unwrap();
        assert_eq!(display.title, "Test Title");
        
        // Test fullscreen toggle
        display.toggle_fullscreen().unwrap();
        assert!(!display.is_fullscreen());
    }

    struct MockAudio {
        spec: Option<AudioSpec>,
        volume: f32,
        paused: bool,
        queue_size: usize,
    }

    impl MockAudio {
        fn new() -> Self {
            Self {
                spec: None,
                volume: 1.0,
                paused: false,
                queue_size: 0,
            }
        }
    }

    impl AudioOutput for MockAudio {
        fn initialize(&mut self, spec: AudioSpec, _format: AudioFormat) -> Result<()> {
            self.spec = Some(spec);
            Ok(())
        }

        fn queue_samples(&mut self, samples: &[u8]) -> Result<()> {
            self.queue_size += samples.len();
            Ok(())
        }

        fn get_queued_size(&self) -> usize {
            self.queue_size
        }

        fn clear_queue(&mut self) -> Result<()> {
            self.queue_size = 0;
            Ok(())
        }

        fn set_volume(&mut self, volume: f32) -> Result<()> {
            self.volume = volume.clamp(0.0, 1.0);
            Ok(())
        }

        fn get_volume(&self) -> f32 {
            self.volume
        }

        fn pause(&mut self, paused: bool) -> Result<()> {
            self.paused = paused;
            Ok(())
        }

        fn is_paused(&self) -> bool {
            self.paused
        }

        fn get_spec(&self) -> Option<&AudioSpec> {
            self.spec.as_ref()
        }
    }

    #[test]
    fn test_mock_audio() {
        let mut audio = MockAudio::new();
        
        // Test initialization
        let spec = AudioSpec::default();
        audio.initialize(spec, AudioFormat::S16).unwrap();
        assert!(audio.get_spec().is_some());
        
        // Test volume
        audio.set_volume(0.5).unwrap();
        assert_eq!(audio.get_volume(), 0.5);
        
        // Test volume clamping
        audio.set_volume(2.0).unwrap();
        assert_eq!(audio.get_volume(), 1.0);
        
        audio.set_volume(-1.0).unwrap();
        assert_eq!(audio.get_volume(), 0.0);
        
        // Test queuing
        audio.queue_samples(&[0; 100]).unwrap();
        assert_eq!(audio.get_queued_size(), 100);
        
        audio.queue_samples(&[0; 50]).unwrap();
        assert_eq!(audio.get_queued_size(), 150);
        
        audio.clear_queue().unwrap();
        assert_eq!(audio.get_queued_size(), 0);
        
        // Test pause
        assert!(!audio.is_paused());
        audio.pause(true).unwrap();
        assert!(audio.is_paused());
    }

    struct MockInput {
        grabbed: bool,
        last_key: Option<KeyCode>,
        last_mouse_pos: (i32, i32),
    }

    impl MockInput {
        fn new() -> Self {
            Self {
                grabbed: false,
                last_key: None,
                last_mouse_pos: (0, 0),
            }
        }
    }

    impl InputHandler for MockInput {
        fn handle_keyboard(&mut self, event: KeyboardEvent) -> Result<()> {
            if event.pressed {
                self.last_key = Some(event.key);
            }
            Ok(())
        }

        fn handle_mouse(&mut self, event: MouseEvent) -> Result<()> {
            if let MouseEvent::Move { x, y, .. } = event {
                self.last_mouse_pos = (x, y);
            }
            Ok(())
        }

        fn grab_input(&mut self, grab: bool) -> Result<()> {
            self.grabbed = grab;
            Ok(())
        }

        fn is_grabbed(&self) -> bool {
            self.grabbed
        }

        fn set_relative_mouse(&mut self, _relative: bool) -> Result<()> {
            Ok(())
        }

        fn warp_mouse(&mut self, x: i32, y: i32) -> Result<()> {
            self.last_mouse_pos = (x, y);
            Ok(())
        }
    }

    #[test]
    fn test_mock_input() {
        let mut input = MockInput::new();
        
        // Test keyboard handling
        let key_event = KeyboardEvent {
            key: KeyCode::A,
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        };
        input.handle_keyboard(key_event).unwrap();
        assert_eq!(input.last_key, Some(KeyCode::A));
        
        // Test mouse handling
        let mouse_event = MouseEvent::Move { x: 100, y: 200, dx: 10, dy: 20 };
        input.handle_mouse(mouse_event).unwrap();
        assert_eq!(input.last_mouse_pos, (100, 200));
        
        // Test input grab
        assert!(!input.is_grabbed());
        input.grab_input(true).unwrap();
        assert!(input.is_grabbed());
        
        // Test mouse warp
        input.warp_mouse(300, 400).unwrap();
        assert_eq!(input.last_mouse_pos, (300, 400));
    }
}