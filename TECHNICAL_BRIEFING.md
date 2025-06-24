# Quickemu Manager - Technical Briefing

## Technology Stack

### Core Framework
- **Dioxus**: Rust-based reactive UI framework
- **Target Platforms**: Desktop (native) with WebUI capability
- **Rendering**: Native rendering via WebView or platform-specific backends

### Language & Tools
- **Language**: Rust (stable channel)
- **Build System**: Cargo with Dioxus CLI
- **Package Format**: Single static binary
- **Target Architectures**: x86_64, aarch64

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                 Dioxus Frontend                 │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────┐ │
│  │ VM Gallery  │ │ VM Details   │ │ Settings │ │
│  │ Component   │ │ Component    │ │ Component│ │
│  └─────────────┘ └──────────────┘ └──────────┘ │
└─────────────────────────────────────────────────┘
                         │
                    State Store
                    (Fermi/Redux)
                         │
┌─────────────────────────────────────────────────┐
│                Backend Services                 │
│  ┌────────────┐ ┌──────────────┐ ┌───────────┐ │
│  │VM Manager  │ │Process       │ │File       │ │
│  │Service     │ │Monitor       │ │Watcher    │ │
│  └────────────┘ └──────────────┘ └───────────┘ │
└─────────────────────────────────────────────────┘
                         │
┌─────────────────────────────────────────────────┐
│              System Integration                 │
│  ┌────────────┐ ┌──────────────┐ ┌───────────┐ │
│  │quickemu/   │ │OS APIs       │ │Display    │ │
│  │quickget    │ │(CPU/RAM)     │ │Protocols  │ │
│  └────────────┘ └──────────────┘ └───────────┘ │
└─────────────────────────────────────────────────┘
```

## Component Specifications

### Frontend Components

#### VM Gallery View
```rust
struct VMGallery {
    vms: Vec<VMSummary>,
    selected: Option<VMId>,
    sort_by: SortCriteria,
}
```
- Grid/List view toggle
- Real-time status indicators
- Resource usage sparklines
- Quick action buttons

#### VM Detail Panel
```rust
struct VMDetail {
    vm: VMConfig,
    metrics: VMMetrics,
    console_url: Option<String>,
}
```
- Full VM configuration display
- Real-time metrics charts
- Console preview/launch
- Action toolbar

### Backend Services

#### VM Manager Service
```rust
trait VMManager {
    async fn discover_vms(&self, path: &Path) -> Result<Vec<VM>>;
    async fn create_vm(&self, template: VMTemplate) -> Result<VM>;
    async fn start_vm(&self, id: VMId) -> Result<Process>;
    async fn stop_vm(&self, id: VMId) -> Result<()>;
    async fn get_vm_status(&self, id: VMId) -> Result<VMStatus>;
}
```

#### Process Monitor
```rust
struct ProcessMonitor {
    processes: HashMap<VMId, Child>,
    metrics: HashMap<VMId, Metrics>,
}
```
- Poll quickemu processes
- Extract resource usage via /proc or system APIs
- Handle process lifecycle events

#### File Watcher
```rust
struct ConfigWatcher {
    watcher: notify::Watcher,
    config_paths: Vec<PathBuf>,
}
```
- Monitor .conf file changes
- Trigger UI updates on modifications
- Handle file creation/deletion

## Data Models

### VM Configuration
```rust
struct VMConfig {
    id: VMId,
    name: String,
    path: PathBuf,
    os: String,
    version: String,
    cpu_cores: u32,
    ram_mb: u32,
    disk_size: String,
    display: DisplayProtocol,
    network: NetworkConfig,
}
```

### VM Metrics
```rust
struct VMMetrics {
    cpu_percent: f32,
    ram_mb_used: u32,
    disk_io_rate: f32,
    network_rate: f32,
    uptime: Duration,
}
```

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
1. Project setup with Dioxus desktop template
2. Basic UI layout and navigation
3. Quickemu .conf parser implementation
4. VM discovery and listing

### Phase 2: Core Features (Week 3-4)
1. VM lifecycle management (start/stop)
2. Process monitoring integration
3. Real-time metrics collection
4. Basic settings management

### Phase 3: Advanced Features (Week 5-6)
1. Quickget integration for VM creation
2. File watching for auto-refresh
3. Display protocol launching
4. Resource usage visualization

### Phase 4: Polish (Week 7-8)
1. Platform-specific styling
2. Performance optimization
3. Error handling and recovery
4. Package and distribution setup

## Technical Challenges & Solutions

### Challenge: Cross-platform Process Management
**Solution**: Use `std::process::Command` with platform-specific adaptors
```rust
#[cfg(target_os = "linux")]
fn launch_display(vm: &VM) -> Result<()> {
    Command::new("remote-viewer")
        .arg(&vm.spice_socket)
        .spawn()?;
}
```

### Challenge: Real-time Metrics without Polling Overhead
**Solution**: Event-driven updates with batched UI refreshes
- Use tokio channels for metric events
- Throttle UI updates to 1Hz
- Cache metrics between updates

### Challenge: WebUI Mode Support
**Solution**: Conditional compilation with feature flags
```toml
[features]
default = ["desktop"]
desktop = ["dioxus-desktop"]
web = ["dioxus-web", "axum"]
```

### Challenge: Single Binary Distribution
**Solution**: Static linking and asset embedding
- Use `include_str!` for assets
- Static musl builds for Linux
- Universal binaries for macOS

## Development Environment

### Prerequisites
- Rust 1.75+ (for Dioxus support)
- Dioxus CLI: `cargo install dioxus-cli`
- Platform SDKs (GTK4 for Linux, Cocoa for macOS)

### Build Commands
```bash
# Development
dx serve --platform desktop

# Release build
dx build --release

# WebUI mode
dx build --features web
```

### Testing Strategy
- Unit tests for parsers and business logic
- Integration tests for quickemu interaction
- UI testing with Dioxus test utilities
- Manual testing on target platforms

## Security Considerations

1. **Process Isolation**: Each VM runs in separate process
2. **File System Access**: Scoped to configured directories
3. **No Elevated Privileges**: Request only when needed
4. **Input Validation**: Sanitize all .conf file inputs
5. **Secure Defaults**: Conservative resource limits

## Performance Targets

- **Startup Time**: < 500ms to first paint
- **Memory Usage**: < 30MB base + 5MB per VM
- **CPU Usage**: < 1% idle, < 5% with monitoring
- **Binary Size**: < 20MB compressed

## Dependencies

### Core Dependencies
```toml
[dependencies]
dioxus = "0.5"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
notify = "6"
sysinfo = "0.30"
```

### Platform-specific
```toml
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"

[target.'cfg(target_os = "linux")'.dependencies]
gtk4 = "0.7"
```