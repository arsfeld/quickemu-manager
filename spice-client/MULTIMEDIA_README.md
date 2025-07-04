# SPICE Client Multimedia Implementation

## Overview

This SPICE client now includes a modular multimedia system that supports video display, audio output, and input handling across multiple platforms.

## Features

- **Generic trait-based API** - Platform-agnostic interfaces for display, audio, and input
- **Multiple backends** - GTK4 and WebAssembly implementations
- **Feature flags** - Only include the backends you need
- **Embeddable** - Can be integrated into existing GTK4 applications
- **Cross-platform executable** - `spice-viewer` binary for standalone use

## Building

### GTK4 backend (default)
```bash
cargo build --release
```

### WebAssembly
```bash
cargo build --target wasm32-unknown-unknown --no-default-features --features backend-wasm
```

## Usage

### Standalone Viewer

Launch QEMU with SPICE:
```bash
qemu-system-x86_64 -spice port=5900,disable-ticketing -vga qxl ...
```

Connect with spice-viewer:
```bash
spice-viewer spice://localhost:5900
```

Options:
- `-f, --fullscreen` - Start in fullscreen mode
- `-W, --width <WIDTH>` - Window width (default: 1024)
- `-H, --height <HEIGHT>` - Window height (default: 768)
- `-p, --password <PASSWORD>` - SPICE password
- `--no-audio` - Disable audio
- `--no-grab` - Disable input grab
- `-t, --title <TITLE>` - Window title

### Embedding in GTK4 Applications

```rust
use spice_client::{Client, ClientBuilder};
use spice_client::multimedia::gtk4::Gtk4Backend;

// Create GTK4 backend with your drawing area
let drawing_area = gtk4::DrawingArea::new();
let backend = Gtk4Backend::with_drawing_area(drawing_area);

// Create and connect client
let client = ClientBuilder::new("spice://localhost:5900")
    .with_multimedia(backend)
    .build()?;

client.connect().await?;
```

## Architecture

The multimedia system consists of three main components:

1. **Display** - Video output and cursor handling
2. **Audio** - Sound playback
3. **Input** - Keyboard and mouse event handling

Each backend implements these traits:
- `Display` - Surface creation, frame presentation, cursor management
- `AudioOutput` - Audio initialization, sample queuing, volume control
- `InputHandler` - Keyboard/mouse events, input grabbing

## Current Status

### Implemented
- ✅ Generic trait system
- ✅ GTK4 backend (display, audio, input)
- ✅ WebAssembly backend stubs
- ✅ Cross-platform executable
- ✅ Feature flag configuration

### TODO
- [ ] Integration with SPICE protocol handlers
- [ ] Video codec support (H.264, VP8/VP9)
- [ ] Advanced audio features (multiple streams, recording)
- [ ] Clipboard integration
- [ ] USB redirection
- [ ] Smartcard support

## Next Steps

1. **Protocol Integration** - Connect multimedia backends to SPICE message handlers
2. **GTK4 Backend** - Implement display using GTK4 DrawingArea and GStreamer for audio
3. **Video Codecs** - Add hardware-accelerated video decoding
4. **WebAssembly** - Complete implementation using Canvas and Web Audio APIs
5. **Testing** - Comprehensive tests for each backend

## Contributing

When adding new backends:
1. Implement the traits in `src/multimedia/<backend>/`
2. Add feature flag to `Cargo.toml`
3. Update the `create_default_backend()` function
4. Add documentation and examples

## License

GPL v3 License - See LICENSE file for details