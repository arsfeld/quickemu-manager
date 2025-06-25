# Quickemu Manager - Technical Briefing

## Technology Stack

### Core Framework
- **GTK4**: Native Rust-based desktop UI framework via gtk4-rs
- **Target Platforms**: Linux (primary), with potential Windows/macOS support
- **Rendering**: Native GTK4 rendering with composite templates

### Language & Tools
- **Language**: Rust (stable channel)
- **Build System**: Cargo with standard Rust toolchain
- **UI Templates**: GTK4 composite templates (.ui files)
- **Package Format**: Single static binary
- **Target Architectures**: x86_64, aarch64

## Architecture Overview

### Minimalist Multi-UI Architecture

Simple architecture with two frontend options sharing core business logic:

```
┌─────────────────────────────────────────────────┐
│              UI Frontends                       │
│  ┌──────────────┐      ┌──────────────┐       │
│  │ GTK4 Desktop │      │ Dioxus Web   │       │
│  │  (Native)    │      │   (WASM)     │       │
│  └──────────────┘      └──────────────┘       │
└─────────────────────────────────────────────────┘
           │                       │
           └───────────┬───────────┘
                       │
┌─────────────────────────────────────────────────┐
│              Core Library                       │
│  ┌────────────┐ ┌──────────────┐ ┌───────────┐ │
│  │VM Manager  │ │Process       │ │Discovery  │ │
│  │            │ │Monitor       │ │           │ │
│  └────────────┘ └──────────────┘ └───────────┘ │
└─────────────────────────────────────────────────┘
                       │
┌─────────────────────────────────────────────────┐
│            System Integration                   │
│  ┌────────────┐ ┌──────────────┐ ┌───────────┐ │
│  │quickemu/   │ │OS APIs       │ │Config     │ │
│  │quickget    │ │(sysinfo)     │ │Parser     │ │
│  └────────────┘ └──────────────┘ └───────────┘ │
└─────────────────────────────────────────────────┘
```

### Simple Project Structure

```
quickemu-manager/
├── src/                     # Current GTK4 implementation
│   ├── main.rs             # GTK4 entry point
│   ├── models/             # Shared data models
│   ├── services/           # Core business logic
│   └── ui/                 # GTK4 UI components
├── dioxus-app/             # Dioxus web frontend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs         # Dioxus entry point
│   │   ├── components/     # Dioxus UI components
│   │   └── api.rs          # Web API client
│   └── index.html          # HTML template
├── Cargo.toml              # Main workspace
└── README.md

## Simple UI Strategy

### 1. GTK4 Desktop (Current)
- **Primary Linux desktop experience**
- Native performance and OS integration
- Direct access to VM management services
- Keep existing implementation as-is

### 2. Dioxus Web Application
- **Browser-based interface**
- Compiles to WASM for client-side execution
- Communicates with simple HTTP API backend
- Cross-platform access via any modern browser

### Implementation Approach

#### Shared Core Logic
```rust
// src/models/vm.rs - Shared between both UIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VM {
    pub id: VMId,
    pub name: String,
    pub config_path: PathBuf,
    pub status: VMStatus,
    pub config: VMConfig,
}

// src/services/vm_manager.rs - Same business logic
impl VMManager {
    pub async fn list_vms(&self) -> Result<Vec<VM>>;
    pub async fn start_vm(&self, id: &VMId) -> Result<()>;
    pub async fn stop_vm(&self, id: &VMId) -> Result<()>;
}
```

#### Dioxus Web Frontend
```rust
// dioxus-app/src/components/vm_card.rs
#[component]
fn VMCard(vm: ReadOnlySignal<VM>) -> Element {
    rsx! {
        div { class: "vm-card",
            div { class: "vm-header",
                h3 { "{vm().name}" }
                span { class: "status-{vm().status}", "{vm().status}" }
            }
            div { class: "vm-actions",
                if vm().status == VMStatus::Stopped {
                    button { 
                        onclick: move |_| start_vm(vm().id),
                        "Start"
                    }
                } else {
                    button { 
                        onclick: move |_| stop_vm(vm().id),
                        "Stop"
                    }
                }
            }
        }
    }
}

// Simple HTTP client for web UI
async fn start_vm(vm_id: VMId) -> Result<()> {
    let response = reqwest::Client::new()
        .post(&format!("/api/vms/{}/start", vm_id))
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to start VM"))
    }
}
```

## Component Specifications

### Core Dioxus Components

#### VM Gallery Widget
```rust
#[derive(CompositeTemplate)]
#[template(resource = "/org/quickemu/Manager/vm_gallery.ui")]
struct VMGallery {
    #[template_child]
    vm_list: TemplateChild<gtk::ListView>,
    #[template_child] 
    view_toggle: TemplateChild<gtk::ToggleButton>,
    
    model: gio::ListStore,
}
```
- GTK4 ListView with custom VM items
- Real-time status indicators via property bindings
- Resource usage with custom Cairo drawing
- Quick action buttons in each row

#### VM Detail Widget  
```rust
#[derive(CompositeTemplate)]
#[template(resource = "/org/quickemu/Manager/vm_detail.ui")]
struct VMDetail {
    #[template_child]
    config_group: TemplateChild<adw::PreferencesGroup>,
    #[template_child]
    metrics_chart: TemplateChild<gtk::DrawingArea>,
    #[template_child]
    action_bar: TemplateChild<gtk::ActionBar>,
}
```
- AdwPreferencesGroup for configuration display
- Custom DrawingArea for real-time metrics
- Console launch via GtkActionBar
- Property bindings for live updates

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

### Challenge: Simple Multi-UI Support
**Solution**: Keep GTK4 app as-is, add minimal Dioxus web frontend
```toml
# Main Cargo.toml (GTK4 app)
[features]
default = []
web-server = ["axum", "tower"]

[dependencies]
# Core dependencies (current)
gtk4 = "0.9"
libadwaita = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
sysinfo = "0.30"

# Optional web server
axum = { version = "0.7", optional = true }
tower = { version = "0.4", optional = true }

# dioxus-app/Cargo.toml (Web frontend)
[dependencies]
dioxus = "0.4"
dioxus-web = "0.4"
serde = { version = "1", features = ["derive"] }
reqwest = { version = "0.11", features = ["json"] }
wasm-bindgen = "0.2"
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
- Platform SDKs:
  - Linux: GTK4 development libraries
  - Web: `wasm-pack` for WASM builds
  - macOS: Xcode Command Line Tools

### Build Commands

#### Current GTK4 Desktop App
```bash
# Development build
cargo build

# Release build
cargo build --release

# With GTK4 features
cargo build --features desktop-gtk
```

#### Dioxus Web Application
```bash
# Build web application (WASM)
cd dioxus-app
dx build --platform web

# Development with hot reload
cd dioxus-app
dx serve --platform web

# Build web server component (optional)
cargo build --features web-server
```

#### Development Workflow
```bash
# Run GTK4 desktop app (current)
cargo run

# Run GTK4 app with web server enabled
cargo run --features web-server

# Run web frontend (separate terminal)
cd dioxus-app && dx serve

# Build both applications
cargo build && cd dioxus-app && dx build
```

### Testing Strategy
- Unit tests for core business logic
- Integration tests for quickemu interaction
- Basic web UI testing with Dioxus
- Manual testing on Linux desktop and web browsers

## Simple Migration Strategy

### Phase 1: Add Web Server Capability (Minimal Change)
1. **Add optional web server** to existing GTK4 app
   - Use feature flag `web-server` to add HTTP API
   - Reuse existing VM management code
   - Simple REST endpoints for basic operations

2. **Keep GTK4 app unchanged**
   - Primary desktop experience remains the same
   - Optional web access for remote management
   - No architectural changes needed

### Phase 2: Create Dioxus Web Frontend
1. **Build simple web interface**
   - Separate `dioxus-app/` directory
   - Basic VM listing and controls
   - Communicate with GTK4 app's web server

2. **Minimal feature set**
   - List VMs and their status
   - Start/stop VM controls
   - Basic VM information display
   - Real-time status updates

### Implementation Benefits
- **Minimal complexity**: Only two UIs to maintain
- **Shared core logic**: VM management code reused directly
- **Independent development**: Web UI can be developed separately
- **Low maintenance**: Simple architecture with clear separation

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

### Main Application (GTK4 Desktop)
```toml
# Cargo.toml
[dependencies]
# Current core dependencies
gtk4 = { version = "0.9", package = "gtk4", features = ["v4_10"] }
libadwaita = { version = "0.7", package = "libadwaita", features = ["v1_6"] }
glib = "0.20"
gio = "0.20"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1.0"
sysinfo = "0.30"
notify = "6"
which = "4.0"

# Optional web server
axum = { version = "0.7", optional = true }
tower = { version = "0.4", optional = true }
tower-http = { version = "0.5", features = ["cors"], optional = true }

[build-dependencies]
glib-build-tools = "0.20"

[features]
default = []
web-server = ["axum", "tower", "tower-http"]
```

### Dioxus Web Frontend
```toml
# dioxus-app/Cargo.toml
[dependencies]
# Minimal Dioxus dependencies
dioxus = "0.4"
dioxus-web = "0.4"
serde = { version = "1", features = ["derive"] }
reqwest = { version = "0.11", features = ["json"] }
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"

# Only the data models needed for web UI
[dependencies.quickemu-types]
path = "../src/models"
features = ["serde"]
```

### Shared Types Only (Minimal)
```toml
# src/models/Cargo.toml (optional separate crate)
[dependencies]
serde = { version = "1", features = ["derive"] }
```