# Console & Remote Access Implementation Guide

This guide documents the implementation of console/remote access support in quickemu-manager, including both VNC and SPICE protocols.

## Overview

The quickemu-manager supports remote console access to VMs through two protocols:
- **VNC (Virtual Network Computing)** - Traditional remote desktop protocol
- **SPICE (Simple Protocol for Independent Computing Environments)** - Modern protocol with better performance

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Web Browser   │────▶│  WebSocket Proxy │────▶│  VM (QEMU)      │
│  (VNC/SPICE     │     │  (Rust Server)   │     │  VNC: 5900-5929 │
│   JS Client)    │     │  Port: 6080+     │     │  SPICE: 5930+   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Implementation Steps

### 1. Protocol Detection System

The VM manager detects which console protocol a VM is using by scanning ports:

```rust
// core/src/services/vm_manager.rs
pub async fn detect_console_port(&self, vm_id: &VMId) -> Result<Option<(u16, ConsoleProtocol)>> {
    // VNC ports: 5900-5929
    for port in 5900..5930 {
        if self.is_port_open("127.0.0.1", port).await {
            return Ok(Some((port, ConsoleProtocol::Vnc)));
        }
    }
    
    // SPICE ports: 5930-5999
    for port in 5930..6000 {
        if self.is_port_open("127.0.0.1", port).await {
            return Ok(Some((port, ConsoleProtocol::Spice)));
        }
    }
    
    Ok(None)
}
```

### 2. Console Protocol Enum

Define the supported protocols:

```rust
// core/src/services/vnc_proxy.rs
#[derive(Debug, Clone, Copy)]
pub enum ConsoleProtocol {
    Vnc,
    Spice,
}

#[derive(Debug, Clone)]
pub struct ConsoleInfo {
    pub websocket_url: String,
    pub auth_token: String,
    pub connection_id: String,
    pub protocol: ConsoleProtocol,
}
```

### 3. WebSocket Proxy Service

The proxy creates a WebSocket bridge between the browser and the VM's console:

```rust
// core/src/services/vnc_proxy.rs
pub struct VncProxy {
    connections: Arc<RwLock<HashMap<String, VncConnection>>>,
    active_proxies: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl VncProxy {
    pub async fn create_connection(
        &self,
        vm_id: String,
        vnc_host: String,
        vnc_port: u16,
    ) -> Result<VncConnection> {
        // 1. Generate connection ID and auth token
        let connection_id = self.generate_connection_id();
        let auth_token = self.generate_auth_token();
        
        // 2. Find available WebSocket port
        let websocket_port = self.find_available_port().await?;
        
        // 3. Start WebSocket proxy
        let proxy_task = tokio::spawn(async move {
            Self::run_websocket_proxy(
                websocket_port,
                vnc_addr,
                auth_token,
                connections_clone,
                connection_id_clone,
            ).await
        });
        
        // 4. Store connection info
        connections.insert(connection_id.clone(), connection);
        
        Ok(connection)
    }
}
```

### 4. Unified Console Component

The console component automatically selects the appropriate viewer:

```rust
// dioxus-app/src/components/vm_console.rs
#[component]
pub fn VmConsole(vm: VM, on_close: EventHandler<()>) -> Element {
    // Start console session
    let console_info = start_vm_console(vm.id, hostname).await?;
    
    rsx! {
        div {
            // Display appropriate viewer based on protocol
            if let Some(info) = console_info() {
                match info.protocol {
                    ConsoleProtocol::Vnc => rsx! {
                        VncViewer {
                            host: ws_host,
                            port: ws_port,
                            auto_connect: true,
                            auth_token: Some(info.auth_token.clone())
                        }
                    },
                    ConsoleProtocol::Spice => rsx! {
                        SpiceViewer {
                            host: ws_host,
                            port: ws_port,
                            auto_connect: true,
                            auth_token: Some(info.auth_token.clone())
                        }
                    }
                }
            }
        }
    }
}
```

### 5. VNC Client Implementation

The VNC client handles the WebSocket connection and VNC protocol:

```rust
// dioxus-app/src/vnc_client.rs
pub struct VncClient {
    websocket: Option<WebSocket>,
    event_tx: mpsc::UnboundedSender<VncEvent>,
    message_rx: mpsc::UnboundedReceiver<VncMessage>,
}

impl VncClient {
    async fn connect(&mut self, url: &str, auth_token: Option<String>) -> Result<(), JsValue> {
        // Create WebSocket
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        
        // Set up event handlers
        let onopen = Closure::wrap(Box::new(move |_| {
            // Send auth token on connection
            if let Some(token) = &auth_token {
                ws.send_with_str(token);
            }
        }));
        
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        
        Ok(())
    }
}
```

### 6. VM Configuration for VNC

Enable VNC when starting VMs:

```rust
// core/src/services/vm_manager.rs
match &vm.config.display {
    DisplayProtocol::Vnc { port } => {
        // Use none display to avoid conflicts
        cmd.arg("--display").arg("none");
        
        // Add VNC server
        let vnc_arg = if *port > 0 {
            format!("-vnc :{}", port - 5900)
        } else {
            "-vnc :0".to_string() // Default to display :0 (port 5900)
        };
        cmd.arg("--extra_args").arg(vnc_arg);
    }
}
```

## Debugging Console Connections

### Enable Debug Logging

The implementation includes comprehensive debug logging:

```rust
// Check console detection
println!("Detecting console port for VM '{}'", vm_id.0);
println!("Found {:?} console on port {} for VM '{}'", protocol, console_port, vm_id.0);

// Monitor WebSocket proxy
log::info!("VNC WebSocket proxy listening on {} for connection {}", ws_addr, connection_id);
log::debug!("WebSocket handshake completed for connection {}", connection_id);

// Track connection status
log::info!("VncClient: WebSocket opened");
log::debug!("VncClient: Received binary message, {} bytes", data.len());
```

### Common Issues and Solutions

1. **"VM does not support console access"**
   - VM is not running
   - VM is using unsupported display protocol
   - Console port is not accessible

2. **"Connection shows as connected then disconnects"**
   - Check if VM has VNC/SPICE enabled
   - Verify port detection is correct
   - Check WebSocket proxy logs

3. **Blank console screen**
   - VNC/SPICE server may not be properly initialized
   - Authentication may have failed
   - Protocol mismatch (trying VNC on SPICE port)

### Testing Console Access

1. **Start VM with VNC**:
   ```bash
   quickemu --vm ubuntu.conf --display none --extra_args "-vnc :0"
   ```

2. **Check if VNC port is open**:
   ```bash
   lsof -i :5900
   netstat -an | grep 5900
   ```

3. **Test with VNC client**:
   ```bash
   vncviewer localhost:5900
   ```

## Future Enhancements

### SPICE Protocol Support (In Progress)

We are actively implementing SPICE support using a custom Rust-based SPICE client library located in the `spice-client/` directory.

#### Current Implementation Status

1. **SPICE Client Library** (`spice-client/`):
   ```rust
   // spice-client/src/client.rs
   pub struct SpiceClient {
       connection: SpiceConnection,
       channels: HashMap<ChannelType, Box<dyn Channel>>,
   }
   
   // Protocol implementation
   - Main channel for control messages
   - Display channel for video streaming
   - Inputs channel for keyboard/mouse
   - Cursor channel for pointer updates
   ```

2. **WebSocket Integration**:
   - The existing `vnc_proxy.rs` can be reused for SPICE
   - Same WebSocket bridge architecture
   - Protocol detection determines VNC vs SPICE handling

3. **Browser Integration** (`dioxus-app/src/spice_client.rs`):
   ```rust
   pub struct SpiceClient {
       // WebAssembly-compatible SPICE client
       // Handles WebSocket connection
       // Renders to HTML5 Canvas
   }
   ```

#### Architecture for SPICE Support

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Web Browser   │────▶│  WebSocket Proxy │────▶│  VM (QEMU)      │
│  (SPICE WASM    │ WS  │  (Protocol-aware)│ TCP │  SPICE Port:    │
│   Client)       │     │  Port: 6080+     │     │  5930-5999      │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                                                  │
        └──────────── SPICE Protocol ─────────────────────┘
```

#### Remaining Work

1. **Complete SPICE Protocol Implementation**:
   - [ ] Finish authentication handshake
   - [ ] Implement image decompression (LZ, JPEG, etc.)
   - [ ] Handle all SPICE message types
   - [ ] Support multiple display channels

2. **WebAssembly Compilation**:
   - [ ] Ensure spice-client compiles to WASM
   - [ ] Optimize for browser performance
   - [ ] Handle browser-specific constraints

3. **Canvas Rendering**:
   - [ ] Efficient frame buffer updates
   - [ ] Hardware acceleration via WebGL
   - [ ] Cursor rendering and tracking

4. **Enhanced Features**:
   - [ ] Clipboard sharing via SPICE agent
   - [ ] Audio playback support
   - [ ] USB redirection (if supported by browser)
   - [ ] Multi-monitor support

### Performance Optimizations

1. **Connection Pooling**: Reuse WebSocket connections
2. **Compression**: Enable VNC/SPICE compression
3. **Adaptive Quality**: Adjust quality based on bandwidth
4. **Hardware Acceleration**: Use WebGL for rendering

## Security Considerations

1. **Authentication**: 
   - Generate secure random tokens
   - Validate tokens before establishing connection
   - Use TLS for WebSocket connections in production

2. **Network Isolation**:
   - Bind console servers to localhost only
   - Use SSH tunneling for remote access
   - Implement rate limiting

3. **Access Control**:
   - Verify user permissions before creating console session
   - Log all console access attempts
   - Implement session timeouts

## References

- [VNC Protocol RFC 6143](https://datatracker.ietf.org/doc/html/rfc6143)
- [SPICE Protocol Documentation](https://www.spice-space.org/spice-protocol.html)
- [noVNC - VNC Client using HTML5](https://github.com/novnc/noVNC)
- [quickemu Documentation](https://github.com/quickemu-project/quickemu)