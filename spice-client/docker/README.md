# Docker Testing Infrastructure

This directory contains the unified Docker setup for E2E testing of the SPICE client.

## Structure

### Dockerfiles
- `Dockerfile.e2e-test` - Native Rust E2E test client
- `Dockerfile.wasm-core-test` - WebAssembly core tests (no browser)
- `Dockerfile.wasm-test` - WebAssembly browser tests (Playwright)
- `Dockerfile.wasm.dev` - Development server with hot-reload
- `Dockerfile.ws-proxy` - WebSocket proxy for WASM clients

### Configuration
- `docker-compose.yml` - Unified compose configuration with profiles
- `supervisord.conf` - Process management for multi-service containers

### Scripts
- `scripts/start-qemu.sh` - QEMU startup script (used by tests/docker/Dockerfile.spice-server)
- `scripts/hot-reload-server.py` - Development server with file watching
- `scripts/watch-and-serve.sh` - Build and serve script for development
- `websocket-proxy.py` - Main WebSocket proxy implementation
- `websocket-proxy-multi.py` - Multi-channel WebSocket proxy

### Directories
- `captures/` - Packet captures from test runs
- `logs/` - Test execution logs

## Usage

See the main [E2E Testing Guide](../E2E_TESTING.md) for usage instructions.

Quick start:
```bash
# From repository root
just test-e2e          # Run default test
just test-e2e-all      # Run all test combinations
./run-e2e-tests.sh -h  # See all options
```