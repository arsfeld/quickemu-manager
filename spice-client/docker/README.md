# QEMU/SPICE Integration Testing

This directory contains Docker configurations for testing the SPICE client with a real QEMU instance.

## Prerequisites

- Docker and Docker Compose
- At least 2GB of free disk space
- Port 5900 available on the host

## Quick Start

1. **Start the QEMU container:**
   ```bash
   docker-compose -f docker-compose.qemu.yml up -d qemu-spice
   ```

2. **Wait for QEMU to be ready:**
   ```bash
   docker-compose -f docker-compose.qemu.yml logs -f qemu-spice
   ```
   
   Look for "Starting QEMU with SPICE server on port 5900"

3. **Run integration tests:**
   ```bash
   # From the spice-client directory
   cargo test --features integration-tests -- --ignored --nocapture
   ```

## Manual Testing

### Connect with a SPICE client:
```bash
# Using spice-gtk client
spicy -h localhost -p 5900

# Using virt-viewer
remote-viewer spice://localhost:5900
```

### Monitor QEMU:
```bash
# Connect to QEMU monitor
telnet localhost 5555

# Useful monitor commands:
(qemu) info spice
(qemu) info vnc
(qemu) info qtree
(qemu) system_powerdown
```

### Debug with VNC:
```bash
# QEMU also exposes VNC on port 5901
vncviewer localhost:5901
```

## Docker Services

### qemu-spice
- QEMU instance with SPICE server enabled
- Runs a minimal Cirros Linux image
- Ports:
  - 5900: SPICE protocol
  - 5901: VNC (for debugging)
  - 5555: QEMU monitor

### spice-client-test
- Runs the Rust SPICE client tests
- Automatically connects to qemu-spice container
- Waits for QEMU to be healthy before starting

## Configuration

Environment variables:
- `SPICE_PORT`: SPICE server port (default: 5900)
- `VNC_PORT`: VNC server port (default: 5901)
- `MONITOR_PORT`: QEMU monitor port (default: 5555)

## Troubleshooting

### Container won't start
```bash
# Check logs
docker-compose -f docker-compose.qemu.yml logs qemu-spice

# Check if ports are in use
netstat -tlnp | grep -E '5900|5901|5555'
```

### SPICE connection fails
```bash
# Verify QEMU is running
docker exec spice-qemu-test ps aux | grep qemu

# Check SPICE status
echo "info spice" | nc localhost 5555
```

### Performance issues
- The container uses software rendering (no GPU acceleration)
- TCG acceleration is slower than KVM
- For better performance, run on a Linux host with KVM support

## Advanced Usage

### Custom VM image
1. Place your image in the `docker/` directory
2. Modify `scripts/download-test-vm.sh` to use your image
3. Rebuild the container

### Enable KVM acceleration (Linux only)
```yaml
# In docker-compose.qemu.yml, add:
devices:
  - /dev/kvm:/dev/kvm
```

Then modify `scripts/start-qemu.sh`:
```bash
-machine pc,accel=kvm
```

### Test specific SPICE features
Modify `scripts/start-qemu.sh` to enable/disable features:
- `-spice gl=on`: Enable OpenGL
- `-spice disable-copy-paste=on`: Disable clipboard
- `-spice playback-compression=off`: Disable audio compression

## Clean Up

```bash
# Stop containers
docker-compose -f docker-compose.qemu.yml down

# Remove volumes
docker-compose -f docker-compose.qemu.yml down -v

# Remove images
docker rmi spice-client-qemu-test spice-client-test
```