# SPICE Web UI Integration Plan
## Quickemu Manager - VM Console Access

### Overview
This plan outlines the integration of SPICE web UI functionality into the Quickemu Manager, enabling users to interact with VMs directly through the web interface without requiring external SPICE clients.

### Architecture Decision

**Chosen Approach: Custom Rust WebSocket Proxy + Optimized SPICE Web Client**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Dioxus Web    │    │   Rust WebSocket │    │   SPICE Server  │
│   Application   │◄──►│      Proxy       │◄──►│   (quickemu)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │
         ▼
┌─────────────────┐
│ spice-web-client│
│   (JavaScript)  │
└─────────────────┘
```

### Core Components

#### 1. Rust WebSocket Proxy Server
**Location**: `core/src/services/spice_proxy.rs`

**Responsibilities**:
- WebSocket-to-TCP bridge for SPICE protocol
- Connection management and lifecycle
- Authentication and security
- Per-VM connection routing
- Error handling and recovery

**Key Features**:
```rust
pub struct SpiceProxyService {
    // Active VM connections
    connections: Arc<RwLock<HashMap<String, SpiceConnection>>>,
    // WebSocket server handle
    server_handle: Option<JoinHandle<()>>,
    // Configuration
    config: SpiceProxyConfig,
}

pub struct SpiceConnection {
    vm_id: String,
    spice_port: u16,
    websocket_port: u16,
    auth_token: String,
    status: ConnectionStatus,
}
```

#### 2. SPICE Web Client Integration
**Location**: `dioxus-app/assets/spice-client/`

**Implementation**:
- Embed optimized spice-web-client (isard-vdi fork)
- Custom wrapper for Dioxus integration
- Canvas-based display rendering
- Keyboard/mouse event handling
- Clipboard integration

#### 3. Dioxus VM Console Component
**Location**: `dioxus-app/src/components/vm_console.rs`

**Features**:
- Full-screen console modal
- Connection status indicators
- Performance metrics overlay
- Touch-friendly controls for mobile
- Keyboard shortcuts

### Implementation Phases

#### Phase 1: Core WebSocket Proxy (Week 1-2)
**Goals**: Establish basic SPICE-to-WebSocket bridge

**Tasks**:
1. **WebSocket Server Setup**
   - Create `SpiceProxyService` in core crate
   - Implement basic TCP-to-WebSocket bridging
   - Add connection lifecycle management
   - Implement graceful shutdown

2. **SPICE Integration**
   - Detect SPICE ports from quickemu processes
   - Parse SPICE authentication tickets
   - Handle SPICE protocol negotiation
   - Implement connection pooling

3. **Security Foundation**
   - Generate secure session tokens
   - Implement token-based authentication
   - Add TLS support for WebSocket connections
   - Basic rate limiting

**Deliverables**:
- Working WebSocket proxy for SPICE connections
- Basic authentication system
- Integration with VM lifecycle management

#### Phase 2: Web Client Integration (Week 3-4)
**Goals**: Embed SPICE client in Dioxus frontend

**Tasks**:
1. **Client Library Setup**
   - Download and customize spice-web-client
   - Create JavaScript wrapper for Dioxus integration
   - Implement connection management
   - Add error handling and reconnection logic

2. **Dioxus Component Development**
   - Create `VmConsole` component
   - Implement modal dialog with full-screen option
   - Add connection status indicators
   - Handle WebSocket connection lifecycle

3. **User Experience**
   - Implement loading states and error messages
   - Add keyboard shortcut support
   - Create mobile-friendly touch controls
   - Implement clipboard integration

**Deliverables**:
- Functional VM console accessible from web UI
- Responsive design for desktop and mobile
- Basic performance monitoring

#### Phase 3: Advanced Features (Week 5-6)
**Goals**: Polish and optimize the console experience

**Tasks**:
1. **Performance Optimization**
   - Implement connection pooling
   - Add compression for WebSocket traffic
   - Optimize JavaScript client performance
   - Memory usage monitoring

2. **Advanced UI Features**
   - Performance metrics overlay (FPS, latency)
   - Connection quality indicators
   - Advanced keyboard shortcuts
   - Screen scaling and zoom controls

3. **Security Hardening**
   - Implement proper session management
   - Add audit logging
   - Enhanced authentication options
   - Network security best practices

**Deliverables**:
- Production-ready SPICE web console
- Performance monitoring and optimization
- Comprehensive security implementation

### Technical Specifications

#### WebSocket Proxy Configuration
```rust
#[derive(Debug, Clone)]
pub struct SpiceProxyConfig {
    pub bind_address: String,
    pub port_range: (u16, u16),
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub auth_timeout: Duration,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
}
```

#### API Endpoints
```rust
// Server functions for console management
async fn start_vm_console(vm_id: String) -> Result<ConsoleInfo, String>;
async fn stop_vm_console(vm_id: String) -> Result<(), String>;
async fn get_console_status(vm_id: String) -> Result<ConsoleStatus, String>;

#[derive(Serialize, Deserialize)]
pub struct ConsoleInfo {
    pub websocket_url: String,
    pub auth_token: String,
    pub connection_id: String,
}
```

#### Client Integration
```javascript
// SPICE client wrapper for Dioxus
class QuickEmuSpiceClient {
    constructor(canvasId, websocketUrl, authToken) {
        this.client = new SpiceMainConn();
        this.canvas = document.getElementById(canvasId);
        this.setupEventHandlers();
    }
    
    connect() {
        // Initialize SPICE connection
        this.client.connect(this.websocketUrl, this.authToken);
    }
    
    disconnect() {
        // Clean shutdown
        this.client.disconnect();
    }
}
```

### Security Considerations

#### Authentication & Authorization
- **Session Tokens**: Cryptographically secure tokens for each console session
- **Time-Limited Access**: Tokens expire after configurable timeout
- **VM-Specific Access**: Tokens tied to specific VM instances
- **User Context**: Integration with future user management system

#### Network Security
- **TLS Encryption**: All WebSocket traffic encrypted
- **Origin Validation**: Restrict connections to authorized origins
- **Rate Limiting**: Prevent abuse and DoS attacks
- **Firewall Integration**: Proper port management and access control

#### Data Protection
- **No Credential Storage**: SPICE tickets handled securely in memory
- **Session Isolation**: Each VM console isolated from others
- **Audit Logging**: Connection attempts and security events logged
- **Secure Defaults**: Conservative security settings by default

### Performance Targets

#### Latency Goals
- **Local Network**: < 50ms input latency
- **Remote Access**: < 200ms input latency
- **Connection Establishment**: < 2 seconds

#### Resource Usage
- **Memory**: < 10MB per active console connection
- **CPU**: < 5% per active console (1080p)
- **Network**: Adaptive bandwidth (50kb/s - 2MB/s)

#### Scalability
- **Concurrent Sessions**: Up to 10 simultaneous console connections
- **Connection Pool**: Efficient reuse of proxy connections
- **Memory Management**: Automatic cleanup of idle connections

### Integration Points

#### VM Lifecycle Integration
```rust
// Extend existing VM management
impl VmService {
    pub async fn get_console_info(&self, vm_id: &str) -> Result<Option<SpiceInfo>> {
        // Extract SPICE port and auth from running VM
    }
    
    pub async fn enable_console_access(&self, vm_id: &str) -> Result<ConsoleInfo> {
        // Start proxy for specific VM
    }
}
```

#### UI Integration Points
1. **VM Card Component**: Add "Console" button to VM cards
2. **VM Detail View**: Integrated console tab/section
3. **Full-Screen Modal**: Dedicated console interface
4. **Status Indicators**: Show console connection status

### Testing Strategy

#### Unit Tests
- WebSocket proxy connection handling
- SPICE protocol message parsing
- Authentication token generation and validation
- Error handling and recovery

#### Integration Tests
- End-to-end console connection flow
- VM lifecycle integration
- Security token validation
- Performance under load

#### Manual Testing
- Cross-browser compatibility
- Mobile device responsiveness
- Network disruption handling
- Security penetration testing

### Deployment Considerations

#### Binary Size Impact
- **Estimated Addition**: +2-3MB to binary (WebSocket proxy code)
- **Asset Size**: +500KB for JavaScript client
- **Total Impact**: Within 30MB target from product guidelines

#### Runtime Dependencies
- **Core**: No new system dependencies
- **Optional**: TLS certificates for secure connections
- **Configuration**: Console settings in existing config system

#### Backwards Compatibility
- **Graceful Degradation**: VMs work without console feature
- **Feature Detection**: UI adapts based on SPICE availability
- **Configuration**: Console feature can be disabled

### Risk Assessment & Mitigation

#### Technical Risks
1. **SPICE Protocol Complexity**
   - *Risk*: Incomplete protocol implementation
   - *Mitigation*: Use proven spice-web-client library

2. **WebSocket Stability**
   - *Risk*: Connection drops and reconnection issues
   - *Mitigation*: Robust reconnection logic and fallback options

3. **Performance Bottlenecks**
   - *Risk*: Poor performance on lower-end systems
   - *Mitigation*: Adaptive quality settings and performance monitoring

#### Security Risks
1. **Network Exposure**
   - *Risk*: Additional attack surface through WebSocket proxy
   - *Mitigation*: Strong authentication, TLS encryption, rate limiting

2. **Session Hijacking**
   - *Risk*: Unauthorized access to VM consoles
   - *Mitigation*: Secure token generation, short expiration times

### Success Metrics

#### Functional Metrics
- **Connection Success Rate**: > 95%
- **Average Connection Time**: < 3 seconds
- **Session Stability**: < 1% unexpected disconnections

#### User Experience Metrics
- **User Adoption**: > 70% of users try console feature
- **User Satisfaction**: Positive feedback on console usability
- **Support Requests**: < 5% related to console functionality

#### Performance Metrics
- **Latency**: Within target ranges for different network conditions
- **Resource Usage**: Within specified memory and CPU limits
- **Scalability**: Support target concurrent connections

### Future Enhancements

#### Post-MVP Features
1. **Multi-Monitor Support**: Handle VMs with multiple displays
2. **Audio Integration**: Enable audio passthrough
3. **File Transfer**: Drag-and-drop file transfer capabilities
4. **Mobile Optimization**: Enhanced touch controls and gestures
5. **Recording**: Screen recording capabilities
6. **Collaboration**: Multi-user console sharing

#### Advanced Security
1. **2FA Integration**: Two-factor authentication for console access
2. **Audit Dashboard**: Comprehensive logging and monitoring
3. **Access Policies**: Fine-grained permission controls
4. **Network Isolation**: Advanced network security features

This integration plan provides a comprehensive roadmap for adding SPICE web console functionality to Quickemu Manager while maintaining the product's core principles of simplicity, performance, and security.