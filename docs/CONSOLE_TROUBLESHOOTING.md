# Console Access Troubleshooting Guide

## Quick Diagnostics

### 1. Check VM Display Protocol

Look at the console output when trying to connect:
```
Detecting console port for VM 'ubuntu-24.04'
Found SPICE port 5930 for VM 'ubuntu-24.04'
```

- **VNC ports**: 5900-5929
- **SPICE ports**: 5930-5999

### 2. Verify VM is Running with Correct Display

**For VNC support**, the VM must be started with VNC enabled:
```bash
# Check current VM process
ps aux | grep qemu | grep vnc

# If not present, restart VM with VNC
quickemu --vm yourvm.conf --display none --extra_args "-vnc :0"
```

**For SPICE support**, quickemu uses SPICE by default:
```bash
# Check for SPICE
ps aux | grep qemu | grep spice
```

### 3. Common Error Messages

#### "VM does not support console access"
**Cause**: No VNC or SPICE port detected
**Solution**: 
1. Ensure VM is running
2. Check VM configuration for display protocol
3. Restart VM with correct display settings

#### "Failed to connect to VNC server"
**Cause**: VNC proxy cannot connect to VNC server
**Solution**:
1. Verify VNC is actually running on the detected port
2. Check firewall rules
3. Try connecting with a standalone VNC client

#### Console shows "Connected" then immediately "Disconnected"
**Cause**: Protocol mismatch or authentication failure
**Solution**:
1. Check debug logs for the actual protocol being used
2. Ensure the VM's display protocol matches what's detected
3. Verify WebSocket proxy is running correctly

## Enable Debug Output

### Server-side (Rust)
The implementation includes debug output by default:
```
VM 'ubuntu-24.04' is running, scanning for VNC ports
Found open port 5930 for VM 'ubuntu-24.04' in VNC range
Creating VNC proxy connection to localhost:5930
```

### Client-side (Browser)
Open browser developer console (F12) to see:
```
VncConsole: Starting console session for VM 'ubuntu-24.04'
VncViewer: Starting VNC connection to ws://localhost:6090
VncClient: WebSocket opened
```

## Quick Fixes

### Force VNC Mode
Edit VM configuration to use VNC:
```bash
# In your VM .conf file
display="none"
# Then start with:
quickemu --vm yourvm.conf --extra_args "-vnc :0"
```

### Test Connection Manually
```bash
# Test VNC port
nc -zv localhost 5900

# Test SPICE port  
nc -zv localhost 5930

# List all QEMU ports
lsof -i -P | grep qemu
```

### Reset Console Session
1. Close the console window
2. Wait 2-3 seconds
3. Try opening console again

## Current Limitations

1. **SPICE viewer in development** - SPICE support is being actively implemented
   - SPICE protocol is detected correctly
   - WebSocket proxy is created successfully
   - Browser-side SPICE client (`spice-client/`) is under development
   - Use native SPICE clients (spicy, remote-viewer) as a temporary workaround
2. **VNC requires manual configuration** - quickemu uses SPICE by default
   - Must configure VM to use VNC display protocol
   - Or use `--extra_args "-vnc :0"` when starting VM
3. **No audio support** - Console access is video-only
4. **Single display only** - Multiple monitors not supported

## Configuring VNC for Web Console

Since quickemu uses SPICE by default, you need to configure VNC:

### Option 1: Modify VM Configuration
Edit your VM's `.conf` file:
```bash
display="none"
```

Then start with:
```bash
quickemu --vm yourvm.conf --extra_args "-vnc :0"
```

### Option 2: Use Native SPICE Client
For VMs using SPICE (default), use:
```bash
spicy -h localhost -p 5930
# or
remote-viewer spice://localhost:5930
```

## Getting Help

When reporting issues, include:
1. VM name and configuration
2. Console error message
3. Server logs showing port detection
4. Browser console logs
5. Output of `ps aux | grep qemu`