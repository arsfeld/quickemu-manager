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

### Independent Multi-UI Architecture

Two independent frontend applications sharing core business logic:

```
┌─────────────────────────────────────────────────┐
│              Independent Frontends              │
│  ┌──────────────┐      ┌──────────────┐       │
│  │ GTK4 Desktop │      │   Dioxus     │       │
│  │  (Native)    │      │ Fullstack    │       │
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

### Project Structure

```
quickemu-manager/
├── core/                    # Shared core library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── models/         # Shared data models
│       │   ├── vm.rs
│       │   ├── config.rs
│       │   └── metrics.rs
│       └── services/       # Core business logic
│           ├── vm_manager.rs
│           ├── parser.rs
│           ├── discovery.rs
│           ├── process_monitor.rs
│           └── metrics.rs
├── gtk4-app/               # GTK4 desktop application
│   ├── Cargo.toml
│   ├── build.rs            # Build script for resources
│   ├── resources/          # GTK4 UI templates
│   │   ├── org.quickemu.Manager.gresource.xml
│   │   └── ui/
│   │       ├── main_window.ui
│   │       ├── vm_card.ui
│   │       └── ...
│   └── src/
│       ├── main.rs         # GTK4 entry point
│       └── ui/             # GTK4 UI components
│           ├── main_window.rs
│           ├── vm_card.rs
│           └── ...
├── dioxus-app/             # Dioxus fullstack application
│   ├── Cargo.toml          # Fullstack dependencies
│   ├── Dioxus.toml         # Dioxus configuration
│   ├── src/
│   │   ├── main.rs         # Fullstack entry point
│   │   ├── components/     # UI components
│   │   ├── server_functions.rs # Server functions
│   │   ├── models/         # Shared data models
│   │   └── pages/          # Page components
│   ├── assets/             # Static assets
│   └── index.html          # HTML template
├── Cargo.toml              # Workspace root
└── README.md

## Simple UI Strategy

### 1. GTK4 Desktop (Current)
- **Primary Linux desktop experience**
- Native performance and OS integration
- Direct access to VM management services
- Keep existing implementation as-is

### 2. Dioxus Fullstack Application

**Decision: Fullstack Implementation**

Using Dioxus 0.6's fullstack capabilities for unified client-server architecture:

- **Fullstack architecture**: Single application with client (WASM) and server components
- **Server functions**: Type-safe RPC calls between client and server
- **Clean separation**: GTK4 handles desktop, Dioxus handles web interface
- **Benefits**:
  - Type-safe communication between frontend and backend
  - Shared data models and business logic
  - Hot reload for both client and server code
  - No separate API layer needed
  - Direct integration with quickemu-core library
- **Deployment options**:
  - Self-contained fullstack server
  - Static site generation for client-only deployment
  - Containerized deployment for easy distribution

### Implementation Approach

#### Shared Core Logic
```rust
// core/src/models/vm.rs - Shared between all UIs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VM {
    pub id: VMId,
    pub name: String,
    pub config_path: PathBuf,
    pub status: VMStatus,
    pub config: VMConfig,
}

// core/src/services/vm_manager.rs - Same business logic
impl VMManager {
    pub async fn list_vms(&self) -> Result<Vec<VM>>;
    pub async fn start_vm(&self, id: &VMId) -> Result<()>;
    pub async fn stop_vm(&self, id: &VMId) -> Result<()>;
}
```

#### Dioxus Fullstack Application
```rust
// dioxus-app/src/main.rs - Fullstack implementation
fn main() {
    // Launch as fullstack application
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

// Server functions for VM operations
#[server(ListVMs)]
pub async fn list_vms() -> Result<Vec<VM>, ServerFnError> {
    // Direct access to quickemu-core on server
    let vm_manager = get_vm_manager().await;
    let vms = get_vms().await;
    
    let mut api_vms = Vec::new();
    for vm in vms.read().await.values() {
        let mut vm_copy = vm.clone();
        vm_manager.update_vm_status(&mut vm_copy).await;
        api_vms.push(convert_vm(vm_copy));
    }
    
    Ok(api_vms)
}

#[server(StartVM)]
pub async fn start_vm(vm_id: String) -> Result<(), ServerFnError> {
    let vm_manager = get_vm_manager().await;
    let vms = get_vms().await;
    let vm_id = VMId(vm_id);
    
    if let Some(vm) = vms.read().await.get(&vm_id) {
        vm_manager.start_vm(vm).await?;
    }
    
    Ok(())
}

// Client-side usage
#[component]
fn HomePage() -> Element {
    let mut vms = use_signal(Vec::<VM>::new);
    
    use_effect(move || {
        spawn(async move {
            if let Ok(vm_list) = list_vms().await {
                vms.set(vm_list);
            }
        });
    });
    
    rsx! {
        div {
            for vm in vms.read().iter() {
                VMCard { vm: vm.clone() }
            }
        }
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

### Challenge: Independent Multi-UI Support
**Solution**: Two independent applications sharing core library
```toml
# gtk4-app/Cargo.toml
[dependencies]
gtk4 = "0.9"
libadwaita = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
quickemu-core = { path = "../core" }

# dioxus-app/Cargo.toml (Fullstack app)
[dependencies]
dioxus = { version = "0.6", features = ["fullstack"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
quickemu-core = { path = "../core" }

# core/Cargo.toml (Shared business logic)
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
sysinfo = "0.30"
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

#### GTK4 Desktop App
```bash
cd gtk4-app

# Development build
cargo build

# Release build
cargo build --release

# Run the app
cargo run
```

#### Dioxus Fullstack Application
```bash
cd dioxus-app

# Install Dioxus CLI if not already installed
cargo install dioxus-cli

# Development with hot reload (fullstack)
dx serve

# Build for production (creates dist/ folder)
dx build --release

# Build and serve
dx serve --release
```

#### Development Workflow
```bash
# Run GTK4 desktop app
cd gtk4-app && cargo run

# Run Dioxus fullstack app with hot reload
cd dioxus-app && dx serve

# Build everything from workspace root
cargo build --workspace

# Build fullstack app for deployment
cd dioxus-app && dx build --release
# Output will be in dioxus-app/dist/

# Deploy the fullstack server
cd dioxus-app && dx serve --release
```

### Testing Strategy
- Unit tests for core business logic
- Integration tests for quickemu interaction
- Basic web UI testing with Dioxus
- Manual testing on Linux desktop and web browsers

## Simple Migration Strategy

### Phase 1: Extract Core Library
1. **Create shared core library**
   - Move VM management logic to `core/` crate
   - Extract models, services, and business logic
   - Make it UI-agnostic

2. **Update GTK4 app**
   - Use shared core library
   - Keep UI code in main crate
   - No functional changes

### Phase 2: Create Dioxus Fullstack Application
1. **Build fullstack application**
   - Separate `dioxus-app/` directory for fullstack UI
   - Client (WASM) and server components in single app
   - Server functions for VM management
   - Direct integration with quickemu-core library

2. **Fullstack-optimized features**
   - List VMs and their status via server functions
   - Start/stop VM controls through type-safe RPC
   - Real-time updates via reactive signals
   - Responsive design for mobile and desktop browsers
   - Progressive Web App capabilities

### Implementation Benefits
- **Complete independence**: GTK4 and fullstack apps are fully separate
- **Type safety**: Shared models between client and server
- **Optimal performance**: Direct function calls instead of HTTP overhead
- **Unified development**: Single app with hot reload for both client and server
- **Clear architecture**: Desktop for local, web for remote management

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

### GTK4 Desktop Application
```toml
# gtk4-app/Cargo.toml
[package]
name = "quickemu-manager-gtk"
version = "0.1.0"
edition = "2021"

[dependencies]
# GTK4 UI dependencies
gtk4 = { version = "0.9", package = "gtk4", features = ["v4_10"] }
libadwaita = { version = "0.7", package = "libadwaita", features = ["v1_6"] }
glib = "0.20"
gio = "0.20"

# Shared core
quickemu-core = { path = "../core" }

# Additional dependencies
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1.0"

[build-dependencies]
glib-build-tools = "0.20"
```

### Dioxus Web Application
```toml
# dioxus-app/Cargo.toml
[dependencies]
# Dioxus web framework
dioxus = { version = "0.4", features = ["web"] }
dioxus-web = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Web-specific dependencies
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Window", "Document", "Element"] }
reqwest = { version = "0.11", features = ["json", "wasm"] }
gloo-timers = { version = "0.3", features = ["futures"] }

# Optional: For connecting to backend API
url = "2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

### Backend API Server (Optional)
```toml
# backend/Cargo.toml
[dependencies]
# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }

# Shared core
quickemu-core = { path = "../core" }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Shared Core Library
```toml
# core/Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1.0"
sysinfo = "0.30"
notify = "6"
which = "4.0"
```