# SPICE Client Testing Plan

## Overview

This document outlines the comprehensive testing strategy for the spice-client library, including unit tests, integration tests, and testing against real SPICE servers.

## Testing Levels

### 1. Unit Tests
- Test individual components in isolation
- Mock external dependencies
- Fast execution, run on every commit
- Target: 80%+ code coverage

### 2. Integration Tests
- Test component interactions
- Use test containers for SPICE servers
- Run in CI/CD pipeline
- Validate protocol compliance

### 3. End-to-End Tests
- Test against real QEMU/SPICE instances
- Manual and automated scenarios
- Performance benchmarks
- Compatibility testing

## Unit Test Structure

### Protocol Module Tests (`src/protocol/tests.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spice_magic_constant() {
        assert_eq!(SPICE_MAGIC, 0x52454451);
    }
    
    #[test]
    fn test_message_serialization() {
        // Test binary serialization/deserialization
    }
    
    #[test]
    fn test_channel_types() {
        // Verify channel type constants
    }
}
```

### Channel Tests (`src/channels/*/tests.rs`)
- Main channel handshake
- Display channel message handling
- Input event encoding
- Error scenarios

### Client Tests (`src/client/tests.rs`)
- Connection state management
- Channel lifecycle
- Event handling
- Reconnection logic

## Integration Test Setup

### Docker-based Test Environment

```dockerfile
# tests/docker/Dockerfile.spice-server
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    qemu-utils \
    spice-client-gtk \
    netcat-openbsd \
    supervisor

# Create test VM image
RUN qemu-img create -f qcow2 /tmp/test.qcow2 1G

# Supervisor config for QEMU with SPICE
COPY supervisord.conf /etc/supervisor/conf.d/

EXPOSE 5900-5999

CMD ["/usr/bin/supervisord"]
```

### Test Harness (`tests/integration/harness.rs`)
```rust
use testcontainers::{Docker, Image};
use spice_client::SpiceClient;

pub struct SpiceServerContainer;

impl Image for SpiceServerContainer {
    type Args = ();
    
    fn descriptor(&self) -> String {
        "spice-test-server:latest".to_string()
    }
    
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("SPICE server is ready")]
    }
}

pub async fn with_spice_server<F>(test: F) 
where
    F: FnOnce(String, u16) -> Fut,
    Fut: Future<Output = ()>,
{
    let docker = Docker::default();
    let container = docker.run(SpiceServerContainer);
    let port = container.get_host_port(5900);
    
    test("localhost".to_string(), port).await;
}
```

## Test Scenarios

### Basic Connectivity Tests
1. **Connection Establishment**
   - Valid connection
   - Invalid host/port
   - Connection timeout
   - Authentication success/failure

2. **Protocol Handshake**
   - Version negotiation
   - Capability exchange
   - Channel setup

### Display Channel Tests
1. **Surface Management**
   - Surface creation
   - Surface updates
   - Resolution changes
   - Multiple displays

2. **Drawing Operations**
   - Rectangle fills
   - Image transfers
   - Cursor updates

### Input Channel Tests
1. **Keyboard Events**
   - Key press/release
   - Modifier keys
   - International keyboards

2. **Mouse Events**
   - Movement
   - Button clicks
   - Wheel scrolling

### Stress Tests
1. **High-frequency Updates**
   - Rapid display changes
   - Input event flooding
   - Memory usage monitoring

2. **Long-duration Tests**
   - Connection stability
   - Memory leaks
   - Resource cleanup

## Mock Infrastructure

### Mock SPICE Server (`tests/mocks/server.rs`)
```rust
pub struct MockSpiceServer {
    listener: TcpListener,
    clients: Vec<MockClient>,
}

impl MockSpiceServer {
    pub async fn new() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        Ok(Self {
            listener,
            clients: Vec::new(),
        })
    }
    
    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }
    
    pub async fn accept_connection(&mut self) -> Result<()> {
        let (stream, _) = self.listener.accept().await?;
        // Handle SPICE protocol...
    }
}
```

## WebAssembly Testing

### Browser-based Tests (`tests/wasm/`)
```javascript
// tests/wasm/spice_client_test.js
import init, { SpiceClient } from '../pkg/spice_client.js';

describe('SPICE Client WASM', () => {
    beforeEach(async () => {
        await init();
    });
    
    it('should create client instance', () => {
        const client = SpiceClient.new("localhost", 5900);
        expect(client).toBeDefined();
    });
    
    it('should handle WebSocket connection', async () => {
        // Test with mock WebSocket server
    });
});
```

### WASM Test Runner
```toml
# wasm-pack.toml
[package]
name = "spice-client-wasm-tests"

[[test]]
name = "web"
```

## Performance Benchmarks

### Benchmark Suite (`benches/`)
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spice_client::protocol::*;

fn benchmark_message_parsing(c: &mut Criterion) {
    c.bench_function("parse SpiceDataHeader", |b| {
        let data = vec![0u8; 16];
        b.iter(|| {
            black_box(parse_header(&data));
        });
    });
}

criterion_group!(benches, benchmark_message_parsing);
criterion_main!(benches);
```

## Real SPICE Server Testing

### QEMU Setup Script (`tests/scripts/setup-qemu.sh`)
```bash
#!/bin/bash
# Create and start a QEMU VM with SPICE

qemu-img create -f qcow2 test-vm.qcow2 10G

qemu-system-x86_64 \
    -hda test-vm.qcow2 \
    -m 2048 \
    -enable-kvm \
    -spice port=5900,disable-ticketing \
    -vga qxl \
    -device virtio-serial \
    -chardev spicevmc,id=vdagent,debug=0,name=vdagent \
    -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 \
    -daemonize
```

### Manual Test Checklist
- [ ] Connect to VM running different OS (Linux, Windows)
- [ ] Test display at various resolutions
- [ ] Verify clipboard sharing (when implemented)
- [ ] Test audio playback (when implemented)
- [ ] Validate USB redirection (when implemented)
- [ ] Test with multiple monitors
- [ ] Verify reconnection after network interruption

## CI/CD Integration

### GitHub Actions Workflow (`.github/workflows/test-spice-client.yml`)
```yaml
name: Test SPICE Client

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cd spice-client
          cargo test --all-features
          
  integration-tests:
    runs-on: ubuntu-latest
    services:
      spice-server:
        image: spice-test-server:latest
        ports:
          - 5900:5900
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cd spice-client
          cargo test --test integration --all-features
          
  wasm-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: |
          curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
          cd spice-client
          wasm-pack test --headless --chrome
```

## Test Data

### Sample Protocol Messages (`tests/data/`)
- Captured SPICE protocol exchanges
- Known-good message sequences
- Error scenarios
- Edge cases

## Coverage Goals

| Component | Target Coverage | Priority |
|-----------|----------------|----------|
| Protocol  | 90%           | High     |
| Client    | 85%           | High     |
| Channels  | 80%           | High     |
| Error Handling | 95%      | Critical |
| WebAssembly | 70%         | Medium   |

## Testing Tools

1. **cargo-tarpaulin** - Code coverage
2. **criterion** - Benchmarking
3. **testcontainers** - Docker integration
4. **wasm-pack** - WebAssembly testing
5. **proptest** - Property-based testing
6. **mockall** - Mocking framework

## Continuous Improvement

1. **Weekly Test Review**
   - Analyze flaky tests
   - Update test scenarios
   - Review coverage reports

2. **Monthly Compatibility Testing**
   - Test against latest QEMU/SPICE
   - Verify browser compatibility
   - Update Docker images

3. **Quarterly Performance Review**
   - Run benchmarks
   - Compare with previous results
   - Optimize hot paths