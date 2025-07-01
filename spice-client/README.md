# spice-client

[![Crates.io](https://img.shields.io/crates/v/spice-client.svg)](https://crates.io/crates/spice-client)
[![Documentation](https://docs.rs/spice-client/badge.svg)](https://docs.rs/spice-client)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/yourusername/spice-client/workflows/CI/badge.svg)](https://github.com/yourusername/spice-client/actions)
[![Coverage Status](https://coveralls.io/repos/github/yourusername/spice-client/badge.svg?branch=main)](https://coveralls.io/github/yourusername/spice-client?branch=main)

A modern, pure Rust implementation of the SPICE (Simple Protocol for Independent Computing Environments) client protocol with WebAssembly support.

## ✨ Features

- 🦀 **Pure Rust** - No C dependencies, easy to build and integrate
- 🚀 **Async/Await** - Modern async API using Tokio and wasm-bindgen
- 🌐 **WebAssembly Support** - Run SPICE clients directly in web browsers  
- 🖥️ **Multi-Platform** - Works on Linux, macOS, Windows, and WASM
- 🔐 **Authentication** - RSA-OAEP (Spice ticket) authentication
- 📺 **Multiple Displays** - Support for multi-monitor setups
- 🎮 **Full Input Support** - Keyboard, mouse, and tablet input modes
- 🎨 **Display Channels** - Hardware accelerated cursor and display updates

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
spice-client = "0.1.0"

# For native builds
tokio = { version = "1", features = ["full"] }

# For WASM builds  
wasm-bindgen = "0.2"
```

### Basic Example

```rust
use spice_client::{SpiceClient, SpiceError};

#[tokio::main]
async fn main() -> Result<(), SpiceError> {
    // Create and connect to a SPICE server
    let mut client = SpiceClient::new("localhost".to_string(), 5900);
    client.connect().await?;
    
    // Start processing events
    client.start_event_loop().await?;
    
    // Get display information
    if let Some(surface) = client.get_display_surface(0).await {
        println!("Display: {}x{}", surface.width, surface.height);
    }
    
    // Send input events
    client.send_mouse_motion(100, 100).await?;
    client.send_key(KeyCode::A, true).await?;
    
    Ok(())
}
```

### WebAssembly Example

```rust
use spice_client::SpiceClient;
use wasm_bindgen_futures::spawn_local;

fn connect_to_spice() {
    spawn_local(async {
        let mut client = SpiceClient::new(
            "ws://localhost:8080/spice".to_string(), 
            0  // Port included in WebSocket URL
        );
        
        match client.connect().await {
            Ok(_) => console_log!("Connected!"),
            Err(e) => console_error!("Failed: {:?}", e),
        }
    });
}
```

## 🏗️ Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   Application   │     │   Web Browser   │
└────────┬────────┘     └────────┬────────┘
         │                       │
    ┌────▼────┐            ┌────▼────┐
    │ Native  │            │  WASM   │
    │ Client  │            │ Client  │
    └────┬────┘            └────┬────┘
         │                       │
    ┌────▼────┐            ┌────▼────┐
    │  Tokio  │            │WebSocket│
    │   TCP   │            │  Proxy  │
    └────┬────┘            └────┬────┘
         │                       │
         └───────┬───────────────┘
                 │
          ┌──────▼──────┐
          │SPICE Server │
          │   (QEMU)    │
          └─────────────┘
```

## 📦 Supported Channels

| Channel | Status | Features |
|---------|--------|----------|
| Main | ✅ Implemented | Connection setup, mouse modes, agent messages |
| Display | ✅ Implemented | Screen updates, drawing operations, streaming |
| Inputs | ✅ Implemented | Keyboard, mouse, tablet input |
| Cursor | ✅ Implemented | Hardware cursor updates |
| Playback | ❌ Not implemented | Audio output |
| Record | ❌ Not implemented | Audio input |
| USB | ❌ Not implemented | USB device redirection |
| Smartcard | ❌ Not implemented | Smartcard redirection |

## 🛠️ Building

### Native Build

```bash
cargo build --release
```

### WebAssembly Build

```bash
# Install wasm-pack if needed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web
wasm-pack build --target web --out-dir pkg

# Build for bundler (webpack, etc.)
wasm-pack build --target bundler --out-dir pkg
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_connection

# Run with logging
RUST_LOG=debug cargo test

# Run WASM tests
wasm-pack test --headless --firefox
```

## 📋 Current Limitations

While functional, this library is still under active development:

- **No audio channels** - Playback/Record not implemented
- **No USB redirection** - USB channel not implemented
- **Limited encryption** - No TLS support (only ticket auth)
- **No clipboard** - Agent clipboard integration pending
- **Basic compression** - Only ZLIB (no LZ4)
- **WebSocket required for WASM** - Cannot use direct TCP from browsers
- **Partial drawing ops** - Some complex QXL operations pending

## 🔧 WebSocket Proxy

For WebAssembly builds, you'll need a WebSocket-to-TCP proxy:

```python
# See examples/websocket-proxy.py
python websocket-proxy.py --listen-port 8080 --spice-host localhost --spice-port 5900
```

## 🚦 Roadmap

- [ ] Audio channel support (Playback/Record)
- [ ] Clipboard integration via agent
- [ ] USB redirection
- [ ] TLS encryption
- [ ] LZ4 compression
- [ ] File transfer support
- [ ] Seamless mouse mode
- [ ] Multi-head support improvements
- [ ] Performance optimizations

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- SPICE Protocol documentation and specification
- The QEMU and SPICE development teams
- The Rust async ecosystem (Tokio, wasm-bindgen)

## 📚 Resources

- [SPICE Protocol Documentation](https://www.spice-space.org/spice-protocol.html)
- [API Documentation](https://docs.rs/spice-client)
- [Examples](examples/)
- [WebSocket Proxy Setup](examples/websocket-proxy.py)

---

<p align="center">Made with ❤️ in Rust</p>