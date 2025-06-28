# SPICE Client WASM Docker Setup

This directory contains Docker configurations for building and testing the WASM version of the SPICE client.

## Quick Start

```bash
# Build and run everything (QEMU + WASM client + WebSocket proxy)
cd spice-client/docker
docker compose -f docker-compose.wasm.yml up --build

# Access the WASM client in your browser
open http://localhost:8888
```

## What's Included

1. **WASM Build Container**: Builds the spice-client as WebAssembly
2. **Nginx Server**: Serves the WASM files and demo HTML page
3. **QEMU Instance**: Runs a VM with SPICE server enabled
4. **WebSocket Proxy**: Bridges WebSocket (from browser) to TCP (SPICE server)

## Files Created

- `Dockerfile.wasm`: Multi-stage build for WASM client
- `docker-compose.wasm.yml`: Full stack composition
- `docker-compose.override.yml`: Development override options

## Usage Scenarios

### Inspect WASM Client
1. Start the stack: `docker compose -f docker-compose.wasm.yml up`
2. Open http://localhost:8888 in your browser
3. Open browser developer tools (F12)
4. Try connecting to the SPICE server using the interface
5. Check console for debug output and errors

### Development Workflow
```bash
# Option 1: Use override for development build
docker compose -f docker-compose.qemu.yml -f docker-compose.override.yml run wasm-dev

# Option 2: Build and test with live reload
docker compose -f docker-compose.wasm.yml up --build
# Make changes to source code
docker compose -f docker-compose.wasm.yml up --build spice-client-wasm
```

### Debugging Tips
- Browser Console: See WASM client logs and errors
- Network Tab: Inspect WebSocket connections
- Docker Logs: `docker compose -f docker-compose.wasm.yml logs -f`

## Architecture
```
Browser (http://localhost:8888)
    ↓
WASM Client (JavaScript/WebAssembly)
    ↓
WebSocket (ws://localhost:5959)
    ↓
WebSocket Proxy Container
    ↓
TCP Socket (qemu-spice:5900)
    ↓
QEMU/SPICE Server Container
```