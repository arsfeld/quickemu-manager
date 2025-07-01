# SPICE Client WASM Architecture

## Executive Summary

The SPICE client has successfully implemented a clean WASM architecture:

✅ **Transport Abstraction** - Unified networking via Transport trait  
✅ **Cross-platform Dependencies** - Using `instant`, `getrandom`, etc.  
✅ **Helper Functions** - Platform utilities in `utils.rs`  
✅ **Minimal Conditionals** - Best files have only 4 platform checks  
✅ **Good Separation** - Platform code is well isolated  

The architecture is production-ready for both native and WebAssembly targets.

## Overview

This document outlines the current WebAssembly (WASM) architecture and remaining improvements:
1. Current implementation status and what's working well
2. Opportunities for further improvement
3. Testing infrastructure recommendations

## Current State Analysis (Updated)

### WASM Implementation Status ✅

The codebase has made significant progress in WASM compatibility:

#### Well-Architected Files (Minimal Conditionals)
- ✅ `src/channels/connection.rs` - **Only 4 conditionals** using Transport trait
- ✅ `src/utils.rs` - Provides cross-platform helpers (sleep, spawn_task)
- ✅ `src/transport.rs` - Clean transport abstraction
- ✅ `src/protocol.rs` - Fully cross-platform
- ✅ `src/video/frame.rs` - Uses `instant::Instant`

#### Files with Higher Conditional Count
- `src/channels/mod.rs` - 14 conditionals (old ChannelConnection implementation)
- `src/client.rs` - Platform-specific implementations
- `src/channels/main.rs` - Core protocol variations
- `src/channels/display.rs` - Different rendering paths

### Key Architectural Improvements Already Made

1. **Transport Abstraction**: Clean `Transport` trait isolates platform differences
2. **Helper Functions**: `utils.rs` provides cross-platform utilities
3. **Cross-platform Dependencies**: Using `instant`, `getrandom`, etc.
4. **Good Separation**: Platform-specific code is mostly isolated

### Remaining Opportunities

1. **Remove Duplication**: Two ChannelConnection implementations exist
2. **Feature Flags**: Could use feature flags instead of target_arch
3. **Further Abstraction**: Some files still have many conditionals

## Current Architecture: Transport Abstraction ✅

The codebase already implements a clean abstraction pattern:

### 1. Networking: Transport Trait Pattern (Already Implemented)

**Current Implementation**: `src/transport.rs` provides a unified interface

```rust
// Transport trait abstracts TCP vs WebSocket
#[async_trait]
pub trait Transport: Send + Sync {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    async fn write(&mut self, buf: &[u8]) -> io::Result<()>;
    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
    async fn flush(&mut self) -> io::Result<()>;
    fn is_connected(&self) -> bool;
    async fn close(&mut self) -> io::Result<()>;
}

// Platform-specific implementations
#[cfg(not(target_arch = "wasm32"))]
pub mod tcp;  // TCP implementation

#[cfg(target_arch = "wasm32")]
pub mod websocket;  // WebSocket implementation
```

**Usage in `channels/connection.rs`**:
```rust
pub struct ChannelConnection {
    transport: Box<dyn Transport>,  // Works with any transport
    // ... other fields
}
```

This pattern successfully reduces platform conditionals to just 4 in the connection module!

### 2. Async Runtime: Use compatible runtime

**Current**: `tokio` (native) vs `wasm-bindgen-futures` (WASM)

**Solution**: 
- Use `async-std` with WASM support
- Or use `tokio` with `tokio_wasm_not_send` feature
- Or lightweight executors like `smol` that work everywhere

### 3. Time/Timers: Cross-platform timing ✅

**Already Implemented**: The codebase uses the `instant` crate

```rust
// In Cargo.toml
instant = { version = "0.1", features = ["wasm-bindgen"] }

// In code (e.g., src/video/frame.rs)
use instant::{Instant, Duration};
```

**Helper functions in `src/utils.rs`**:
```rust
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}
```

### 4. Rendering: Keep existing structure

- Native: Keep current implementation
- WASM: Keep `canvas` rendering in `channels/display_wasm.rs`
- Just reduce the conditionals in shared code

### 5. File Organization (Keep existing structure)

```
src/
├── client.rs          # Minimize #[cfg] by using cross-platform crates
├── channels/
│   ├── mod.rs        # Reduce conditionals
│   ├── display.rs    # Shared display logic
│   └── display_wasm.rs # WASM-specific rendering (keep as-is)
└── protocol.rs       # Already platform-agnostic
```

## Implementation Plan (Simplified)

### Phase 1: Audit and Replace Platform-Specific Crates

1. **Identify all `#[cfg]` usage**:
   ```bash
   rg "#\[cfg\(.*wasm" --type rust
   ```

2. **Replace with cross-platform alternatives**:
   
   | Current | Replacement | Notes |
   |---------|-------------|-------|
   | `tokio::net::TcpStream` | `gloo-net::websocket::WebSocket` | Use WebSocket for both |
   | `std::time::Instant` | `instant::Instant` | Drop-in replacement |
   | `tokio::time::sleep` | `async-std::task::sleep` | Or `gloo-timers` |
   | Platform-specific spawn | `wasm_bindgen_futures::spawn_local` | Unified spawning |

### Phase 2: Gradual Migration (File by File)

Start with the easiest files first:

1. **`src/protocol.rs`** - Already platform-agnostic ✓
2. **`src/video.rs`** - Replace timer conditionals with `instant`
3. **`src/client_shared.rs`** - Unify spawn/sleep functions
4. **`src/client.rs`** - Biggest change: unified networking

Example migration:
```rust
// Before: src/client.rs
#[cfg(not(target_arch = "wasm32"))]
async fn connect_tcp(host: &str, port: u16) -> Result<TcpStream> {
    TcpStream::connect((host, port)).await
}

#[cfg(target_arch = "wasm32")]
async fn connect_websocket(url: &str) -> Result<WebSocket> {
    // WebSocket connection
}

// After: src/client.rs
async fn connect(url: &str) -> Result<impl AsyncRead + AsyncWrite> {
    // Use a crate that provides unified interface
    let socket = unified_websocket::connect(url).await?;
    Ok(socket)
}
```

### Phase 3: Keep Platform-Specific Code Isolated

Some code MUST remain platform-specific:

1. **Rendering**: 
   - Keep `display_wasm.rs` for Canvas rendering
   - Keep native rendering separate
   - Use a simple feature flag in `channels/mod.rs`

2. **Binary targets**:
   - `spice-test-client` remains native-only
   - WASM entry points stay in `lib.rs` with `#[wasm_bindgen]`

### Phase 4: Update Dependencies

```toml
[dependencies]
# Cross-platform deps (no more target-specific sections needed!)
instant = "0.1"
gloo-net = "0.4"
gloo-timers = "0.3"
futures = "0.3"
async-trait = "0.1"

# These remain platform-specific
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1", features = ["full"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = "0.3"
```

## WASM Test Infrastructure

We implement a two-tier testing approach:

### Tier 1: Core Protocol Tests (Lightweight)
- **No browser required** - Uses Node.js with WebAssembly directly
- **Fast execution** - Tests only protocol and networking layers
- **Frequently run** - Part of regular CI/CD pipeline
- Tests:
  - WebSocket connections
  - Protocol handshakes
  - Message serialization/deserialization
  - Channel establishment
  - Connection state management

### Tier 2: Full Integration Tests (Browser-based)
- **Uses Playwright** - Full browser environment
- **Tests rendering** - Canvas, display updates, UI interactions
- **Less frequent** - Run for releases or major changes
- Tests:
  - Display rendering
  - Mouse/keyboard input through DOM
  - Canvas operations
  - Full client lifecycle

### 1. WebSocket Proxy Enhancement

Update the existing `websocket-proxy.py` to support multiple simultaneous connections:

```python
# docker/websocket-proxy-multi.py
class SpiceWebSocketProxy:
    def __init__(self):
        self.connections = {}
        
    async def handle_connection(self, websocket, path):
        # Support multiple SPICE channels
        channel_type = self.parse_channel_type(path)
        target_port = self.get_channel_port(channel_type)
        # ... proxy implementation
```

### 2. WASM Test Runner

Create a WASM test runner that can execute in a headless browser:

```javascript
// tests/wasm/test-runner.js
const { chromium } = require('playwright');

async function runWasmTests() {
    const browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    
    // Load WASM test module
    await page.goto('http://localhost:8080/test.html');
    
    // Execute tests
    const results = await page.evaluate(async () => {
        const wasm = await import('/spice_client.js');
        return await wasm.run_all_tests();
    });
    
    console.log('Test results:', results);
    await browser.close();
}
```

### 3. Docker Compose for WASM E2E Tests

```yaml
# docker/docker-compose.wasm-e2e.yml
version: '3.8'

services:
  # SPICE server (same as native tests)
  qemu-spice:
    image: spice-test-server
    ports:
      - "5900:5900"
    
  # WebSocket proxy for all SPICE channels
  websocket-proxy:
    build:
      context: .
      dockerfile: Dockerfile.ws-proxy
    environment:
      - SPICE_HOST=qemu-spice
      - MAIN_CHANNEL_PORT=5900
      - DISPLAY_CHANNEL_PORT=5901
      - INPUTS_CHANNEL_PORT=5902
      - CURSOR_CHANNEL_PORT=5903
    ports:
      - "8080:8080"  # Main WebSocket
      - "8081:8081"  # Display WebSocket
      - "8082:8082"  # Inputs WebSocket
      - "8083:8083"  # Cursor WebSocket
    depends_on:
      - qemu-spice
    
  # WASM test runner
  wasm-test-runner:
    build:
      context: ..
      dockerfile: docker/Dockerfile.wasm-test
    volumes:
      - ../pkg:/app/pkg
      - ../tests/wasm:/app/tests
    environment:
      - WS_MAIN_URL=ws://websocket-proxy:8080
      - WS_DISPLAY_URL=ws://websocket-proxy:8081
      - WS_INPUTS_URL=ws://websocket-proxy:8082
      - WS_CURSOR_URL=ws://websocket-proxy:8083
    depends_on:
      - websocket-proxy
    command: npm test
```

### 4. WASM E2E Test Implementation

```rust
// tests/wasm/e2e_test.rs
#[wasm_bindgen_test]
async fn test_wasm_spice_connection() {
    // Initialize console logging
    console_log::init_with_level(log::Level::Debug).unwrap();
    
    // Connect to SPICE server through WebSocket proxy
    let client = spice_client::wasm::connect(
        "ws://localhost:8080",
        "test-canvas"
    ).await.unwrap();
    
    // Run connection test
    client.wait_for_init().await.unwrap();
    
    // Send test inputs
    client.send_key_event(KeyCode::A, true).await.unwrap();
    client.send_mouse_move(100, 100).await.unwrap();
    
    // Verify display updates
    let frame = client.capture_frame().await.unwrap();
    assert!(!frame.is_empty());
    
    client.disconnect().await.unwrap();
}
```

## Benefits of Simplified Approach

1. **Minimal Changes**: Keep existing code structure, just swap dependencies
2. **Proven Solutions**: Use battle-tested cross-platform crates
3. **Faster Migration**: No need to redesign the entire architecture
4. **Lower Risk**: Changes are localized and can be done incrementally
5. **Better Ecosystem**: Leverage existing Rust/WASM ecosystem

## Migration Strategy

1. **Start Small**: Begin with simple utilities (timers, instant)
2. **Test Continuously**: Use the WASM core tests after each change
3. **One File at a Time**: Complete migration file-by-file
4. **Keep What Works**: Don't change platform-specific code that's already working well

## Dependencies Update

Update Cargo.toml to better separate platform dependencies:

```toml
[dependencies]
# Core dependencies (used by all platforms)
binrw = "0.13"
thiserror = "1.0"
log = "0.4"

# Native dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.35", features = ["full"] }
native-tls = "0.2"

# WASM dependencies  
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "WebSocket", "MessageEvent", "ErrorEvent",
    "CanvasRenderingContext2d", "ImageData"
]}
js-sys = "0.3"

[dev-dependencies]
wasm-bindgen-test = "0.3"
playwright = "1.40"  # For WASM browser testing
```

## Recommendations for Further Improvement

### 1. Remove Duplicate ChannelConnection
The codebase has two ChannelConnection implementations:
- `src/channels/mod.rs` - Old implementation with 14+ conditionals
- `src/channels/connection.rs` - New implementation with Transport trait (only 4 conditionals)

**Action**: Remove the old implementation from mod.rs and update all channels to use connection.rs

### 2. Consider Feature Flags
Instead of `#[cfg(target_arch = "wasm32")]`, consider using feature flags:
```toml
[features]
default = ["native"]
native = ["tokio/full"]
wasm = ["wasm-bindgen", "web-sys"]
```

### 3. Reduce Conditionals in Remaining Files
Focus on files with higher conditional counts:
- `src/client.rs` - Consider using the Transport pattern here too
- `src/channels/main.rs` - Abstract platform differences
- `src/channels/display.rs` - Separate rendering logic more clearly

### 4. Performance Testing
- Benchmark Transport trait overhead
- Compare native TCP vs WebSocket performance
- Profile WASM bundle size

## Conclusion

The SPICE client demonstrates excellent WASM architecture with:
- Clean abstractions (Transport trait)
- Minimal platform conditionals where it matters
- Good use of cross-platform crates
- Well-organized code structure

The codebase is a good example of how to build a dual-target (native + WASM) Rust application.