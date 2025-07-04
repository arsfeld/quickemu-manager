# SPICE Client Multimedia Architecture Guide

## Overview

This guide outlines the architecture for implementing multimedia capabilities (video, audio, input) in the SPICE client library. The design emphasizes:

- **Platform Independence**: Generic traits that work across all platforms
- **Modular Backends**: GTK4 and WebAssembly implementations via feature flags
- **Embeddable**: Can be integrated into GTK4 applications
- **QEMU Compatibility**: Can replace spice-gtk as a VM viewer

## Architecture Design

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Layer                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ spice-viewer│  │ GTK4 App     │  │ WASM Browser    │   │
│  │ (executable)│  │ (embedding)  │  │ (web client)    │   │
│  └─────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                    SPICE Client Library                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               Generic Multimedia API                  │   │
│  │  ┌──────────┐  ┌──────────┐  ┌────────────────┐    │   │
│  │  │  Display  │  │  Audio   │  │  Input Handler │    │   │
│  │  │  Trait    │  │  Trait   │  │  Trait         │    │   │
│  │  └──────────┘  └──────────┘  └────────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
│                              │                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Backend Implementations                  │   │
│  │  ┌──────────┐  ┌────────────────┐                   │   │
│  │  │   GTK4   │  │  WebAssembly   │                   │   │
│  │  │ (default)│  │  (feature)     │                   │   │
│  │  └──────────┘  └────────────────┘                   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Feature Flags

```toml
[features]
default = ["backend-gtk4"]  # Default to GTK4 for native desktop support
backend-gtk4 = ["dep:gtk4", "dep:gdk4"]
backend-wasm = ["dep:web-sys", "dep:wasm-bindgen"]
```

### Generic Traits

```rust
// Display trait for video output
pub trait Display: Send + Sync {
    fn create_surface(&mut self, width: u32, height: u32) -> Result<()>;
    fn present_frame(&mut self, data: &[u8], format: PixelFormat) -> Result<()>;
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    fn set_cursor(&mut self, cursor: Option<CursorData>) -> Result<()>;
}

// Audio trait for sound output
pub trait AudioOutput: Send + Sync {
    fn initialize(&mut self, spec: AudioSpec) -> Result<()>;
    fn queue_samples(&mut self, samples: &[u8]) -> Result<()>;
    fn set_volume(&mut self, volume: f32) -> Result<()>;
    fn pause(&mut self, paused: bool) -> Result<()>;
}

// Input handler trait
pub trait InputHandler: Send + Sync {
    fn handle_keyboard(&mut self, event: KeyboardEvent) -> Result<()>;
    fn handle_mouse(&mut self, event: MouseEvent) -> Result<()>;
    fn grab_input(&mut self, grab: bool) -> Result<()>;
}
```

## Implementation Plan

### Phase 1: Core Infrastructure
1. Define generic multimedia traits
2. Update Cargo.toml with feature flags
3. Create module structure for backends

### Phase 2: GTK4 Backend
1. Implement Display using GTK4 DrawingArea/GL
2. Implement AudioOutput using GStreamer
3. Implement InputHandler for GTK4 events
4. Ensure embeddability in GTK4 apps

### Phase 3: Executable
1. Create spice-viewer binary
2. Command-line argument parsing
3. QEMU integration (URI handling)
4. Fullscreen and window management

### Phase 4: WebAssembly Support
1. Implement Display using Canvas API
2. Implement AudioOutput using Web Audio API
3. Implement InputHandler for browser events
4. Update build configuration

## Usage Examples

### As a standalone viewer
```bash
# Launch QEMU
qemu-system-x86_64 -spice port=5900,disable-ticketing ...

# Connect with our viewer
spice-viewer spice://localhost:5900
```

### Embedded in GTK4 application
```rust
use spice_client::{Client, multimedia::gtk4::Gtk4Backend};

let backend = Gtk4Backend::new(drawing_area);
let client = Client::builder()
    .with_multimedia(backend)
    .build()?;

client.connect("spice://localhost:5900").await?;
```

### Feature Selection
```toml
# For GTK4 build (default)
spice-client = "0.1"

# For WASM build
spice-client = { version = "0.1", default-features = false, features = ["backend-wasm"] }
```

## Benefits

1. **Modularity**: Backends are completely separate, reducing binary size
2. **Platform Native**: Each backend uses optimal libraries for its platform
3. **WASM Compatible**: No native dependencies leak into WASM builds
4. **Embeddable**: Can be integrated into existing GTK4 applications
5. **Drop-in Replacement**: Can replace spice-gtk for VM viewing

## Next Steps

1. Implement the generic trait system
2. Create GTK4 backend as the reference implementation
3. Build the spice-viewer executable
4. Optimize GTK4 backend performance
5. Complete WASM backend for web deployment