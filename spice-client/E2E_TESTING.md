# E2E Testing Guide

This guide explains the unified end-to-end testing system for the SPICE client.

## Quick Start

```bash
# Run default test (native client + debug server)
just test-e2e

# Test specific combinations
just test-e2e-native      # Native client + debug server
just test-e2e-wasm-core   # WASM core + debug server
just test-e2e-qemu        # Native client + QEMU server

# Run all tests (full matrix)
just test-e2e-all

# Run essential tests only
just test-e2e-essential
```

## Test Matrix

The E2E testing system supports the following combinations:

| Implementation | Debug Server | QEMU Server |
|---------------|--------------|-------------|
| Native        | ✓            | ✓           |
| WASM Core     | ✓            | ✓           |

### Implementations

- **Native**: Rust binary client
- **WASM Core**: WebAssembly without browser (protocol testing)

### Servers

- **Debug**: Lightweight SPICE test server (spice-master test-display-no-ssl)
- **QEMU**: Full QEMU instance with SPICE enabled

## Advanced Usage

The test runner script supports various options:

```bash
# Show help
./run-e2e-tests.sh --help

# Test with custom duration
./run-e2e-tests.sh native debug --duration 60

# Keep containers running after tests
./run-e2e-tests.sh native debug --keep

# Verbose output
./run-e2e-tests.sh native debug --verbose

# Dry run (show what would be executed)
./run-e2e-tests.sh all all --dry-run
```

## Docker Architecture

The unified system uses Docker Compose profiles:

```yaml
# docker/docker-compose.yml
profiles:
  - server-debug     # Debug test server
  - server-qemu      # QEMU server
  - test-native      # Native test client
  - test-wasm-core   # WASM core tests
```

## What Gets Tested

Each E2E test validates:

- ✓ TCP/WebSocket connection establishment
- ✓ SPICE protocol handshake
- ✓ Main channel initialization
- ✓ Display channel connection and surface updates
- ✓ Input channel (mouse and keyboard events)
- ✓ Cursor channel updates
- ✓ Proper channel synchronization

## Test Results

Test results and logs are saved to `test-results/`:

```bash
test-results/
├── native_debug.log          # Successful test logs
├── native_qemu_failure.log   # Failed test logs
└── ...
```

## Debugging Failed Tests

1. **Check logs**: Failed tests automatically save logs to `test-results/`
2. **Run with verbose mode**: `./run-e2e-tests.sh native debug -v`
3. **Keep containers running**: `./run-e2e-tests.sh native debug -k`
4. **Check container logs**: `docker logs <container-name>`

## CI/CD Integration

The tests integrate with GitHub Actions:

```yaml
- name: Run E2E tests
  run: just test-e2e-essential
```

## Development

For development with hot-reload:

```bash
# Start WASM development environment
just wasm-dev

# Manual testing against local server
just test-spice localhost 5900
```

## Cleanup

```bash
# Clean up all test containers
just test-e2e-clean

# Full cleanup (including build artifacts)
just clean
```