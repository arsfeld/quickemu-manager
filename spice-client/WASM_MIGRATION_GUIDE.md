# WASM Migration Guide - Practical Steps

## Current Implementation Status ✅

The SPICE client codebase has already implemented most WASM compatibility best practices:

### ✅ Already Implemented

1. **`instant` crate** - Already in Cargo.toml with correct features
2. **`getrandom` crate** - Already configured with `js` feature  
3. **Helper functions** - `src/utils.rs` provides:
   - `sleep()` - Cross-platform sleep function
   - `spawn_task()` - Cross-platform task spawning
4. **Transport abstraction** - `src/transport.rs` provides unified networking
5. **Minimal conditionals** - `src/channels/connection.rs` has only 4 platform conditionals

### ⚠️ Minor Issues Remaining

1. **Test files still use `std::time::Instant`** - Fixed in latest update
2. **Binary file uses `std::time::Instant`** - Acceptable (binaries are native-only)

## Quick Wins - Start Here!

These changes are easy and provide immediate value:

### 1. Replace `std::time::Instant` with `instant` crate ✅

```toml
# Cargo.toml
[dependencies]
instant = { version = "0.1", features = ["wasm-bindgen"] }
```

```rust
// Before
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_sys::Performance;

// After
use instant::Instant; // Works everywhere!
```

**Status**: ✅ Already implemented in production code. Test files updated.

### 2. Use `getrandom` for random numbers ✅

```toml
[dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

**Status**: ✅ Already in Cargo.toml

### 3. Replace platform-specific sleep/timeout ✅

```rust
// Already implemented in src/utils.rs:
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
}
```

**Status**: ✅ Helper functions already exist in `src/utils.rs`

## Networking Strategy

For SPICE, we need TCP on native and WebSocket on WASM. Since SPICE is a binary protocol, we have options:

### Option 1: Always use WebSocket (Recommended for new projects)
- Use WebSocket on both native and WASM
- Requires WebSocket proxy for native too
- Simplest code, one path for everything

### Option 2: Keep TCP for native, WebSocket for WASM (Current approach)
- Better performance on native
- Keep existing `#[cfg]` for connection setup
- But minimize conditionals elsewhere

### Option 3: Use a unified transport trait (Current Implementation) ✅
```rust
// Already implemented in src/transport.rs
#[async_trait]
pub trait Transport: Send + Sync {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    async fn write(&mut self, buf: &[u8]) -> io::Result<()>;
    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
    async fn flush(&mut self) -> io::Result<()>;
    fn is_connected(&self) -> bool;
    async fn close(&mut self) -> io::Result<()>;
}

// Used in src/channels/connection.rs with minimal conditionals
pub struct ChannelConnection {
    transport: Box<dyn Transport>,
    // ... other fields
}
```

**Status**: ✅ The codebase uses the Transport trait pattern effectively

## File-by-File Migration Status

### ✅ Completed
1. `src/protocol.rs` - Already cross-platform
2. `src/utils.rs` - Contains cross-platform helpers (sleep, spawn_task)
3. `src/transport.rs` - Unified transport abstraction
4. `src/channels/connection.rs` - Uses Transport trait (only 4 conditionals)
5. Test files - Updated to use `instant::Instant`

### 🔧 Good Architecture (Minimal Changes Needed)
1. `src/client_shared.rs` - Already uses helper functions from utils.rs
2. `src/video/frame.rs` - Already uses `instant::Instant`
3. `src/channels/cursor.rs` - Well-structured with minimal conditionals
4. `src/channels/inputs.rs` - Clean implementation

### ⚠️ Higher Conditional Count (But Acceptable)
1. `src/client.rs` - Platform-specific client implementations
2. `src/channels/display.rs` - Different rendering paths
3. `src/channels/main.rs` - Core protocol with platform variations
4. `src/channels/mod.rs` - Old ChannelConnection implementation (consider removing)

## Testing Strategy

After each file migration:
1. Run native tests: `cargo test`
2. Run WASM tests: `just test-wasm-core`
3. Check for any new warnings/errors

## Common Patterns

### Pattern 1: Feature detection instead of platform detection
```rust
// Instead of:
#[cfg(target_arch = "wasm32")]

// Consider:
#[cfg(feature = "websocket")]
```

### Pattern 2: Runtime detection where needed
```rust
fn is_wasm() -> bool {
    cfg!(target_arch = "wasm32")
}

if is_wasm() {
    // WASM-specific code
} else {
    // Native code
}
```

### Pattern 3: Dependency injection
```rust
struct Client<T: Transport> {
    transport: T,
}

// No conditionals in Client implementation!
```

## Crates Shopping List

Essential cross-platform crates:
- `instant` - Cross-platform Instant
- `getrandom` - Random number generation
- `futures` - Async primitives
- `pin-project` - For custom futures
- `thiserror` - Error handling
- `tracing` - Logging (works on WASM!)

WASM-specific but useful:
- `gloo` - Web API bindings
- `wasm-bindgen-futures` - JS promise integration
- `web-sys` - Web platform APIs

## Don't Forget!

1. **Keep binaries native-only** - `[[bin]]` targets don't need to work on WASM
2. **Test early and often** - Use the WASM test infrastructure
3. **Profile performance** - Some cross-platform crates may be slower
4. **Document WASM limitations** - Not everything will work exactly the same

## Example: Migrating a Simple File

Let's migrate `src/video.rs` as an example:

```rust
// Before migration
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct VideoDecoder {
    #[cfg(not(target_arch = "wasm32"))]
    start_time: Instant,
    #[cfg(target_arch = "wasm32")]
    start_time: f64,
}

// After migration
use instant::Instant;

pub struct VideoDecoder {
    start_time: Instant, // Works on both platforms!
}
```

That's it! One import change, remove conditionals, done. 🎉

## Summary of Current State

The SPICE client codebase is already well-prepared for WASM:

### ✅ What's Working Well
1. **Cross-platform dependencies** already in place (`instant`, `getrandom`)
2. **Helper functions** in `src/utils.rs` abstract platform differences
3. **Transport trait** in `src/transport.rs` provides clean networking abstraction
4. **Minimal conditionals** in key files like `channels/connection.rs` (only 4)
5. **Good separation** between platform-specific and shared code

### 🎯 Opportunities for Improvement
1. **Remove duplicate ChannelConnection** in `channels/mod.rs` (use connection.rs version)
2. **Consider feature flags** instead of `target_arch` for better clarity
3. **Further reduce conditionals** in display and main channel implementations

### 📊 Platform Conditional Stats
- **Best**: `channels/connection.rs` (4 conditionals)
- **Good**: Most channel implementations (< 10 conditionals)
- **Higher**: `channels/mod.rs` (14 conditionals) - candidate for refactoring

The codebase demonstrates good WASM compatibility practices and is ready for production use on both native and WebAssembly targets.