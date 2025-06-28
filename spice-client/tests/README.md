# SPICE Client Testing Guide

## Quick Start

### Running All Tests

```bash
# Run complete test suite (unit + integration)
./run-integration-tests.sh

# Run only unit tests
./run-integration-tests.sh --unit-only

# Run only integration tests (assumes server is running)
./run-integration-tests.sh --integration-only

# Run specific test
./run-integration-tests.sh test_connect_to_spice_server
```

### Manual Testing with Real SPICE Server

1. **Start a QEMU VM with SPICE**:
```bash
qemu-system-x86_64 \
    -machine pc \
    -m 2048 \
    -spice port=5900,disable-ticketing \
    -vga qxl \
    -display none
```

2. **Run the tests**:
```bash
export SPICE_TEST_HOST=localhost
export SPICE_TEST_PORT=5900
export SPICE_INTEGRATION_TESTS=1
cargo test --test integration
```

### Docker-based Testing

The test suite includes a Docker container that runs a minimal QEMU instance with SPICE:

```bash
# Build and start the test server
cd tests/docker
docker-compose up -d

# Check if server is ready
docker exec spice-test-server nc -z localhost 5900

# View server logs
docker logs spice-test-server

# Stop the server
docker-compose down
```

## Test Structure

### Unit Tests
Located in `src/*/tests.rs`:
- Protocol serialization/deserialization
- Message handling
- Error conditions
- State management

### Integration Tests
Located in `tests/integration/`:
- Connection establishment
- Channel negotiation
- Display surface handling
- Authentication flows

### Mock Server Tests
Located in `tests/mocks/`:
- Mock SPICE server implementation
- Testing without Docker/QEMU
- Protocol compliance testing

## Environment Variables

- `SPICE_TEST_HOST` - SPICE server hostname (default: localhost)
- `SPICE_TEST_PORT` - SPICE server port (default: 5900)
- `SPICE_INTEGRATION_TESTS` - Set to "1" to enable integration tests
- `RUST_LOG` - Log level (debug, info, warn, error)
- `SKIP_CLEANUP` - Set to "true" to keep Docker containers running

## Debugging Failed Tests

### Enable Debug Logging
```bash
export RUST_LOG=debug
cargo test -- --nocapture
```

### Keep Docker Container Running
```bash
./run-integration-tests.sh --skip-cleanup
# Then inspect the container
docker exec -it spice-test-server bash
```

### View SPICE Server Logs
```bash
./run-integration-tests.sh --logs
```

### Run Tests in Verbose Mode
```bash
./run-integration-tests.sh --verbose
```

## Writing New Tests

### Unit Test Example
```rust
#[test]
fn test_my_feature() {
    // Test implementation
}
```

### Integration Test Example
```rust
#[tokio::test]
async fn test_spice_feature() -> Result<(), SpiceError> {
    // Skip if not in integration environment
    if std::env::var("SPICE_INTEGRATION_TESTS").is_err() {
        return Ok(());
    }
    
    // Test implementation
}
```

### Mock Server Test Example
```rust
#[tokio::test]
async fn test_with_mock_server() -> Result<()> {
    let config = MockServerConfig::default();
    let server = MockSpiceServer::new(config).await?;
    
    // Test implementation
}
```

## CI/CD Integration

Tests run automatically in GitHub Actions:
- On every push to main/master
- On pull requests
- Can be triggered manually

View test results at: `.github/workflows/spice-client-tests.yml`

## Troubleshooting

### Docker Issues
- Ensure Docker daemon is running
- Check Docker compose version: `docker-compose --version`
- Verify port 5900 is not in use: `lsof -i :5900`

### Connection Timeouts
- Increase timeout: `TEST_TIMEOUT=600 ./run-integration-tests.sh`
- Check firewall settings
- Verify SPICE server is actually running

### WASM Tests Failing
- Ensure Chrome/Firefox is installed
- Update wasm-pack: `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`
- Check browser console for errors