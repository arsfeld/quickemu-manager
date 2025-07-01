# WASM Architecture - Quick Reference

## Overview

We've documented a complete plan to improve the SPICE client's WebAssembly port by:
1. Eliminating conditional compilation scattered throughout the codebase
2. Creating a clean abstraction layer with platform-specific implementations
3. Setting up comprehensive WASM E2E testing infrastructure

## Key Files Created

### Architecture Documentation
- `WASM_ARCHITECTURE.md` - Complete architectural plan and implementation guide

### WebSocket Proxy Infrastructure
- `docker/websocket-proxy-multi.py` - Multi-channel WebSocket-to-TCP proxy
- `docker/Dockerfile.ws-proxy` - Updated to support multi-channel proxy
- `docker/docker-compose.wasm-e2e.yml` - WASM E2E test orchestration

### WASM Testing
- `docker/Dockerfile.wasm-test` - WASM test runner container
- `tests/wasm/package.json` - Node.js dependencies for WASM tests
- `tests/wasm/test-runner.js` - Playwright-based WASM E2E test runner

### Build Commands
```bash
# Run lightweight WASM core tests (recommended - fast!)
just test-wasm-core

# Run full WASM tests with browser (slower, comprehensive)
just test-wasm-full

# Run essential E2E tests (native + WASM core)
just test-e2e-essential

# Run all E2E tests (native + WASM core + WASM full)
just test-e2e-all

# Clean up containers
just test-wasm-core-clean
just test-wasm-full-clean
```

## Architecture Summary

### Current Issues
- 13 files with `#[cfg(target_arch = "wasm32")]` conditionals
- Platform-specific code using incompatible crates
- Different async runtimes for native vs WASM

### Simplified Solution
Use cross-platform crates instead of creating abstractions:

| Problem | Solution |
|---------|----------|
| `tokio::net::TcpStream` vs WebSocket | Use `gloo-net` or similar |
| `std::time::Instant` differences | Use `instant` crate |
| Different async runtimes | Use `wasm-bindgen-futures` everywhere |
| Timer differences | Use `gloo-timers` |

### Example Migration
```rust
// Before - lots of conditionals
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(target_arch = "wasm32")]
use web_sys::WebSocket;

// After - one import works everywhere
use gloo_net::websocket::WebSocket;
```

## WebSocket Proxy Features

The multi-channel proxy (`websocket-proxy-multi.py`) supports:
- Separate WebSocket connections for each SPICE channel
- Real-time connection statistics via WebSocket
- Flexible routing with multiple URL formats
- Performance optimizations (TCP_NODELAY, keep-alive)

## WASM E2E Testing

The WASM test infrastructure includes:
- Headless Chrome execution via Playwright
- WebSocket proxy for SPICE protocol translation
- Comprehensive test suite covering:
  - Basic connections
  - Multi-channel setup
  - Input events (keyboard/mouse)
  - Display updates
  - Reconnection handling

## Next Steps

1. **Review the simplified plan** in `WASM_ARCHITECTURE.md`
2. **Start with easy wins** - Replace `instant`, timers first
3. **Migrate incrementally** - One file at a time
4. **Test continuously** - Use `just test-wasm-core`

## Benefits of Simplified Approach

- **Minimal refactoring** - Keep existing code structure
- **Proven solutions** - Use established cross-platform crates
- **Faster delivery** - No need to redesign everything
- **Lower risk** - Changes are localized
- **Community support** - Leverage existing crates