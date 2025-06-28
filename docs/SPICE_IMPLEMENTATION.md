# SPICE Client Implementation Guide

This document tracks the ongoing implementation of SPICE protocol support in quickemu-manager.

## Project Structure

```
quickemu-manager/
├── spice-client/          # Rust SPICE client library
│   ├── src/
│   │   ├── client.rs      # Main SPICE client
│   │   ├── protocol.rs    # SPICE protocol definitions
│   │   ├── channels/      # Channel implementations
│   │   │   ├── main.rs    # Main control channel
│   │   │   ├── display.rs # Display/video channel
│   │   │   ├── inputs.rs  # Keyboard/mouse channel
│   │   │   └── cursor.rs  # Cursor channel
│   │   └── video.rs       # Video decoders
│   └── examples/          # Test applications
└── dioxus-app/
    └── src/
        ├── spice_client.rs       # WASM wrapper
        └── components/
            └── spice_viewer.rs   # UI component

```

## Current Status

### ✅ Completed
- Protocol detection (SPICE ports 5930-5999)
- WebSocket proxy infrastructure
- UI framework for SPICE viewer
- Basic project structure
- Error handling and user feedback

### 🚧 In Progress
- SPICE protocol implementation in `spice-client/`
- WebAssembly compilation setup
- Canvas rendering integration

### 📋 TODO
- Complete SPICE handshake
- Implement video decompression
- Handle all SPICE message types
- Input event handling
- Clipboard support
- Audio playback

## Implementation Plan

### Phase 1: Basic Connection (Current)
```rust
// spice-client/src/client.rs
impl SpiceClient {
    pub async fn connect(&mut self, host: &str, port: u16) -> Result<()> {
        // 1. TCP connection
        let stream = TcpStream::connect((host, port)).await?;
        
        // 2. SPICE handshake
        self.send_red_link_mess().await?;
        self.receive_red_link_reply().await?;
        
        // 3. Main channel setup
        self.setup_main_channel().await?;
        
        Ok(())
    }
}
```

### Phase 2: Display Channel
```rust
// spice-client/src/channels/display.rs
impl DisplayChannel {
    pub async fn handle_message(&mut self, msg: SpiceMessage) -> Result<()> {
        match msg.type {
            SPICE_MSG_DISPLAY_MODE => self.handle_display_mode(msg).await?,
            SPICE_MSG_DISPLAY_MARK => self.handle_display_mark(msg).await?,
            SPICE_MSG_DISPLAY_DRAW_FILL => self.handle_draw_fill(msg).await?,
            SPICE_MSG_DISPLAY_DRAW_COPY => self.handle_draw_copy(msg).await?,
            // ... other drawing commands
        }
        Ok(())
    }
}
```

### Phase 3: WebAssembly Integration
```rust
// dioxus-app/src/spice_client.rs
#[cfg(target_arch = "wasm32")]
impl SpiceClient {
    pub fn new() -> Self {
        // Initialize WASM-compatible client
    }
    
    pub async fn connect_websocket(&mut self, url: &str) -> Result<()> {
        // Connect via WebSocket instead of TCP
    }
    
    pub fn render_to_canvas(&self, canvas_id: &str) -> Result<()> {
        // Render frame buffer to HTML5 canvas
    }
}
```

## Testing Strategy

### 1. Unit Tests
```bash
cd spice-client
cargo test
```

### 2. Integration Tests
```bash
# Test with local QEMU VM
qemu-system-x86_64 -spice port=5930,disable-ticketing -vnc :1
cargo run --example spice_connect -- localhost:5930
```

### 3. Browser Testing
```bash
cd dioxus-app
dx serve --port 8080
# Open browser and test console connection
```

## Protocol References

- [SPICE Protocol Specification](https://www.spice-space.org/spice-protocol.html)
- [spice-protocol headers](https://gitlab.freedesktop.org/spice/spice-protocol)
- [QEMU SPICE documentation](https://www.qemu.org/docs/master/system/invocation.html#hxtool-6)

## Development Tips

### Debugging SPICE Traffic
```bash
# Capture SPICE traffic
tcpdump -i lo -w spice.pcap 'port 5930'

# Analyze with Wireshark (has SPICE dissector)
wireshark spice.pcap
```

### QEMU SPICE Options
```bash
# Basic SPICE with no authentication
qemu-system-x86_64 -spice port=5930,disable-ticketing

# SPICE with WebSocket support (QEMU 7.0+)
qemu-system-x86_64 -spice port=5930,websocket=5931,disable-ticketing

# SPICE with compression
qemu-system-x86_64 -spice port=5930,image-compression=auto-glz,zlib-glz-wan-compression=always
```

### Building for WebAssembly
```toml
# Cargo.toml additions for WASM
[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["WebSocket", "MessageEvent", "BinaryType"] }

[lib]
crate-type = ["cdylib", "rlib"]
```

## Known Challenges

1. **Binary Protocol Parsing**: SPICE uses little-endian binary protocol
2. **Image Compression**: Multiple algorithms (LZ, JPEG, ZLIB)
3. **State Management**: Complex state machine for channels
4. **Performance**: Real-time video streaming in browser
5. **WASM Limitations**: No direct TCP sockets, must use WebSocket

## Next Steps

1. Complete basic SPICE handshake in `spice-client`
2. Test with simple SPICE server
3. Implement minimal display channel
4. Create WASM build configuration
5. Integrate with existing WebSocket proxy
6. Update SpiceViewer component to use real client