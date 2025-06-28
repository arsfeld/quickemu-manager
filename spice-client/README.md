# spice-client

[![Crates.io](https://img.shields.io/crates/v/spice-client.svg)](https://crates.io/crates/spice-client)
[![Documentation](https://docs.rs/spice-client/badge.svg)](https://docs.rs/spice-client)
[![License](https://img.shields.io/crates/l/spice-client.svg)](https://github.com/yourusername/spice-client)

A pure Rust implementation of the SPICE (Simple Protocol for Independent Computing Environments) client protocol. This crate provides both native and WebAssembly support for connecting to SPICE servers, commonly used for remote desktop access to virtual machines.

## Features

- 🦀 **Pure Rust** - No C dependencies, works seamlessly with Rust's async ecosystem
- 🌐 **WebAssembly Support** - Run SPICE clients directly in web browsers
- 🔌 **Async/Await** - Built on Tokio for efficient async I/O
- 📺 **Display Channel** - Support for receiving and decoding display updates
- ⌨️ **Input Channel** - Keyboard and mouse input support
- 🔐 **Authentication** - SPICE ticket-based authentication
- 🎥 **Video Streaming** - H.264 and VP8/VP9 codec support (optional)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
spice-client = "0.1.0"
```

For WebAssembly support:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
spice-client = { version = "0.1.0", features = ["wasm"] }
```

## Quick Start

### Native Example

```rust
use spice_client::{SpiceClient, SpiceError};

#[tokio::main]
async fn main() -> Result<(), SpiceError> {
    // Create a new SPICE client
    let mut client = SpiceClient::new("localhost".to_string(), 5900);
    
    // Connect to the SPICE server
    client.connect().await?;
    
    // Start the event loop
    client.start_event_loop().await?;
    
    // Get display surface when available
    if let Some(surface) = client.get_display_surface(0).await {
        println!("Display size: {}x{}", surface.width, surface.height);
        // Process surface.data as needed
    }
    
    Ok(())
}
```

### WebAssembly Example

```rust
use spice_client::{SpiceClient, SpiceError};
use wasm_bindgen_futures::spawn_local;

pub fn connect_to_spice(host: String, port: u16) {
    spawn_local(async move {
        let mut client = SpiceClient::new(host, port);
        
        if let Err(e) = client.connect().await {
            log::error!("Failed to connect: {:?}", e);
            return;
        }
        
        client.start_event_loop().await.unwrap();
    });
}
```

## Architecture

The crate is organized into several modules:

- **`client`** - Main SpiceClient implementation
- **`protocol`** - SPICE protocol definitions and message types
- **`channels`** - Individual channel implementations (main, display, inputs, etc.)
- **`video`** - Video codec support and decoding
- **`error`** - Error types and handling

## Protocol Support

Currently implemented:
- ✅ Main channel (initialization and capabilities)
- ✅ Display channel (basic drawing commands)
- ✅ Inputs channel (keyboard/mouse events)
- 🚧 Cursor channel (work in progress)
- 🚧 Audio playback channel (planned)
- 🚧 USB redirection (planned)

## Building for WebAssembly

To build for WebAssembly:

```bash
# Install wasm-pack if you haven't already
cargo install wasm-pack

# Build the WebAssembly module
wasm-pack build --target web --features wasm
```

## Examples

Check out the `examples/` directory for more detailed examples:

- `native_client.rs` - Full-featured native SPICE client
- `web_viewer.rs` - WebAssembly-based SPICE viewer
- `dioxus_integration.rs` - Integration with Dioxus web framework

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/spice-client
cd spice-client

# Run tests
cargo test

# Run with examples
cargo run --example native_client
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- SPICE Protocol documentation from the [spice-space.org](https://www.spice-space.org/) project
- Inspired by [spice-gtk](https://gitlab.freedesktop.org/spice/spice-gtk) and other SPICE implementations

## Status

⚠️ **This crate is under active development and not yet feature-complete.** The API may change significantly before reaching 1.0. Use in production at your own risk.

### Roadmap to 1.0

- [ ] Complete display channel implementation
- [ ] Full authentication support
- [ ] Video codec integration
- [ ] Comprehensive test coverage
- [ ] Performance optimizations
- [ ] API stabilization