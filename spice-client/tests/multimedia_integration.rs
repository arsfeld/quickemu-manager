#![cfg(feature = "backend-gtk4")]

use spice_client::multimedia::{
    self,
    audio::{AudioFormat, AudioOutput},
    display::{Display, DisplayMode, PixelFormat},
    input::{InputHandler, KeyCode, LegacyKeyboardEvent, MouseButton, MouseEvent},
    AudioSpec, MultimediaBackend,
};

#[test]
#[ignore] // Ignore by default as it requires display
fn test_gtk4_backend_creation() {
    // This test requires GTK4 to be installed
    let backend = multimedia::create_default_backend();
    assert!(
        backend.is_ok(),
        "Failed to create GTK4 backend: {:?}",
        backend.err()
    );
}

#[test]
#[ignore] // Ignore by default as it requires display
fn test_gtk4_display_creation() {
    let backend = multimedia::create_default_backend().unwrap();
    let display = backend.create_display();
    assert!(
        display.is_ok(),
        "Failed to create GTK4 display: {:?}",
        display.err()
    );
}

#[test]
#[ignore] // Ignore by default as it requires audio device
fn test_gtk4_audio_creation() {
    let backend = multimedia::create_default_backend().unwrap();
    let audio = backend.create_audio();
    assert!(
        audio.is_ok(),
        "Failed to create GTK4 audio: {:?}",
        audio.err()
    );
}

#[test]
#[ignore] // Ignore by default as it requires display
fn test_gtk4_input_creation() {
    let backend = multimedia::create_default_backend().unwrap();
    let input = backend.create_input();
    assert!(
        input.is_ok(),
        "Failed to create GTK4 input: {:?}",
        input.err()
    );
}

#[test]
fn test_rgb_frame_data_generation() {
    let width = 320;
    let height = 240;
    let format = PixelFormat::Rgb888;
    let bytes_per_pixel = format.bytes_per_pixel();

    // Generate test pattern
    let mut frame_data = vec![0u8; width * height * bytes_per_pixel];

    for y in 0..height {
        for x in 0..width {
            let offset = (y * width + x) * bytes_per_pixel;
            // Red gradient horizontally
            frame_data[offset] = (x * 255 / width) as u8;
            // Green gradient vertically
            frame_data[offset + 1] = (y * 255 / height) as u8;
            // Blue checkerboard
            frame_data[offset + 2] = if (x / 10 + y / 10) % 2 == 0 { 255 } else { 0 };
        }
    }

    assert_eq!(frame_data.len(), width * height * bytes_per_pixel);
}

#[test]
fn test_audio_sample_generation() {
    let spec = AudioSpec {
        frequency: 44100,
        channels: 2,
        samples: 1024,
    };

    let format = AudioFormat::S16;
    let bytes_per_sample = format.bytes_per_sample();
    let duration_ms = 100;
    let samples_needed = (spec.frequency as usize * duration_ms / 1000) * spec.channels as usize;
    let buffer_size = samples_needed * bytes_per_sample;

    // Generate sine wave
    let mut audio_data = vec![0u8; buffer_size];
    let frequency = 440.0; // A4 note

    for i in 0..samples_needed / spec.channels as usize {
        let time = i as f32 / spec.frequency as f32;
        let value = (time * frequency * 2.0 * std::f32::consts::PI).sin();
        let sample = (value * 32767.0) as i16;

        let offset = i * spec.channels as usize * bytes_per_sample;
        // Write to all channels
        for ch in 0..spec.channels {
            let ch_offset = offset + (ch as usize * bytes_per_sample);
            audio_data[ch_offset..ch_offset + bytes_per_sample]
                .copy_from_slice(&sample.to_le_bytes());
        }
    }

    assert_eq!(audio_data.len(), buffer_size);
}

#[test]
fn test_keyboard_event_creation() {
    let events = vec![
        LegacyKeyboardEvent {
            key: KeyCode::A,
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        },
        LegacyKeyboardEvent {
            key: KeyCode::Escape,
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        },
        LegacyKeyboardEvent {
            key: KeyCode::F11,
            pressed: true,
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
        },
    ];

    for event in events {
        match event.key {
            KeyCode::A => assert!(event.pressed),
            KeyCode::Escape => assert!(event.pressed),
            KeyCode::F11 => assert!(event.pressed),
            _ => panic!("Unexpected key code"),
        }
    }
}

#[test]
fn test_mouse_event_creation() {
    let events = vec![
        MouseEvent::Motion {
            x: 100,
            y: 200,
            relative_x: 10,
            relative_y: -5,
        },
        MouseEvent::Button {
            button: MouseButton::Left,
            pressed: true,
            x: 150,
            y: 250,
        },
        MouseEvent::Wheel {
            delta_x: 0,
            delta_y: -120,
        },
    ];

    for event in events {
        match event {
            MouseEvent::Motion { x, y, .. } => {
                assert!(x >= 0);
                assert!(y >= 0);
            }
            MouseEvent::Button { button, .. } => {
                assert!(matches!(button, MouseButton::Left));
            }
            MouseEvent::Wheel { delta_y, .. } => {
                assert_ne!(delta_y, 0);
            }
        }
    }
}
