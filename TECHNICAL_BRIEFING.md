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
│  │ GTK4 Desktop │      │    Dioxus    │       │
│  │  (Native)    │      │(Web+Desktop) │       │
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
├── dioxus-app/             # Multi-platform application
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs         # Platform-aware entry point
│   │   ├── components/     # UI components
│   │   ├── api.rs          # Web API client
│   │   └── services/       # Platform wrappers
│   └── index.html          # HTML template
├── Cargo.toml              # Workspace root
└── README.md

## Simple UI Strategy

### 1. GTK4 Desktop (Current)
- **Primary Linux desktop experience**
- Native performance and OS integration
- Direct access to VM management services
- Keep existing implementation as-is

### 2. Dioxus Multi-Platform Application
- **Single codebase for web and desktop**
- Desktop mode: Native window using WebView (no GTK4 dependencies)
- Web mode: Standalone server with browser access
- Cross-platform: Linux, Windows, macOS support

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

#### Dioxus Multi-Platform App
```rust
// dioxus-app/src/main.rs - Desktop & Web modes
fn main() {
    // Configure for desktop or web based on features
    #[cfg(feature = "desktop")]
    dioxus_desktop::launch(app);
    
    #[cfg(feature = "web")]
    dioxus_web::launch(app);
    
    #[cfg(feature = "server")]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(launch_server());
}

#[component]
fn app() -> Element {
    let vm_manager = use_context_provider(|| Arc::new(VMManager::new()));
    let vms = use_signal(|| vec![]);
    
    rsx! {
        Router::<Route> {}
    }
}

// Platform-agnostic VM operations
async fn start_vm(vm_id: String) -> Result<()> {
    #[cfg(not(feature = "server"))]
    {
        // Desktop mode - direct VM management
        let manager = use_context::<Arc<VMManager>>();
        manager.start_vm(&vm_id).await
    }
    
    #[cfg(feature = "server")]
    {
        // Web mode - API calls
        api::start_vm(&vm_id).await
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

# dioxus-app/Cargo.toml (Multi-platform app)
[dependencies]
dioxus = { version = "0.4", features = ["desktop", "web", "fullstack"] }
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

#### Dioxus Multi-Platform Application
```bash
cd dioxus-app

# Desktop mode (native window)
cargo run --features desktop

# Web server mode
cargo run --features server

# Build for desktop
cargo build --release --features desktop

# Build for web (WASM)
dx build --platform web

# Development with hot reload
dx serve --platform desktop  # or web
```

#### Development Workflow
```bash
# Run GTK4 desktop app
cd gtk4-app && cargo run

# Run Dioxus desktop app
cd dioxus-app && cargo run --features desktop

# Run Dioxus web server
cd dioxus-app && cargo run --features server

# Build all applications from workspace root
cargo build --workspace

# Build specific app
cargo build -p quickemu-manager-gtk
cargo build -p quickemu-manager-ui --features desktop
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

### Phase 2: Create Dioxus Multi-Platform App
1. **Build multi-platform application**
   - Separate `dioxus-app/` directory
   - Desktop mode for native experience
   - Web server mode for browser access
   - Uses same core library as GTK4 app

2. **Full feature parity**
   - List VMs and their status
   - Start/stop VM controls
   - VM creation and configuration
   - Real-time status updates
   - Works on Linux, Windows, macOS

### Implementation Benefits
- **Complete independence**: Each UI can run without the other
- **Shared core logic**: VM management code reused directly
- **Independent development**: UIs can be developed and deployed separately
- **No GTK4 dependencies**: Web UI builds without desktop libraries

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

### Dioxus Multi-Platform App
```toml
# dioxus-app/Cargo.toml
[dependencies]
# Dioxus framework
dioxus = "0.4"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

# Shared core business logic
quickemu-core = { path = "../core" }

# Platform-specific dependencies
dioxus-desktop = { version = "0.4", optional = true }
dioxus-web = { version = "0.4", optional = true }
dioxus-fullstack = { version = "0.4", optional = true }
axum = { version = "0.7", optional = true }

[features]
default = ["desktop"]
desktop = ["dioxus-desktop"]
web = ["dioxus-web"]
server = ["dioxus-fullstack", "axum"]
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