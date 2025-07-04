# ⚡ Quickemu Manager

A modern VM management application built with Rust, featuring both a GTK4 desktop interface and a cross-platform web interface.

## ✨ Features

- **Cross-platform** - Native GTK4 desktop app for Linux + web interface for all platforms
- **Auto-discovery** - Automatically finds existing VMs in common directories
- **VM Management** - Start, stop, and monitor your virtual machines
- **Modern UI** - Clean interface built with GTK4 and Dioxus
- **Memory Safe** - Written in Rust for reliability  

## 🏗️ Architecture

**Two Applications Sharing a Common Core**

```
🖥️  GTK4 Desktop App     🌐  Multi-Platform Web App
     (Native Linux)              (Web + Desktop)
            │                           │
            └─────────┬─────────────────┘
                      │
              🦀 Shared Rust Core
          (VM Management, Metrics, Discovery)
                      │
              ⚙️  System Integration
         (quickemu/quickget, OS APIs, Config)
```

### Application Options

**GTK4 Desktop** - Native Linux desktop application  
**Dioxus Multi-Platform** - Cross-platform desktop and web application  
**Shared Core** - Both applications use the same Rust backend  

## 🚀 Getting Started

### Prerequisites
- 🦀 Rust 1.75+
- ⚡ [quickemu](https://github.com/quickemu-project/quickemu) installed
- 🎯 [quickget](https://github.com/quickemu-project/quickget) (optional, for VM creation)

### 🏃‍♂️ Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/quickemu-manager.git
cd quickemu-manager

# 🖥️ Build GTK4 Desktop App
cd gtk4-app
cargo build --release
cargo run

# 🌐 Build Dioxus Multi-Platform App
cd ../dioxus-app

# Desktop mode (native window)
cargo run --features desktop

# Web server mode (access from any browser)
cargo run --features server

# Build for deployment
cargo build --release --features desktop
```

### 🎨 Development

```bash
# GTK4 app development
cd gtk4-app && cargo run

# Dioxus app with hot reload ⚡
cd dioxus-app
dx serve --platform desktop  # Desktop development
dx serve --platform web      # Web development

# Build everything 💪
cargo build --workspace
```

## 🌟 Features

### 🎯 VM Discovery
Auto-finds VMs in:
- `~/.config/quickemu/`
- `~/VMs/`
- Custom directories you specify

### 🎮 VM Control
- **Start/Stop** - VM lifecycle management
- **Display Launch** - Console access
- **Configuration** - View VM configuration
- **Status Monitoring** - Track VM status

## 🛠️ System Dependencies

### 🐧 Linux (GTK4 App)
**Fedora/RHEL:**
```bash
sudo dnf install gtk4-devel libadwaita-devel
```

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libadwaita
```

### 🌐 Web App
The Dioxus web app requires no additional system dependencies beyond Rust.

## 🏃‍♂️ Usage

### 🎬 First Launch
1. App scans for existing VMs automatically
2. VMs are displayed in a grid layout
3. Status indicators show current VM states

### 🎮 Daily Workflow
- Start/stop VMs with click actions
- Monitor VM status updates
- Launch VM consoles
- Navigate with keyboard shortcuts

### 🔧 Configuration
Settings auto-sync across platforms:
- **Linux**: `~/.config/quickemu-manager/config.toml`
- **macOS**: `~/Library/Application Support/quickemu-manager/config.toml`
- **Windows**: `%APPDATA%\quickemu-manager\config.toml`

## 🏗️ Tech Stack

### 🦀 Core Foundation
- **Rust** - Memory safety and performance
- **Tokio** - Async runtime
- **Serde** - Serialization

### 🖥️ GTK4 Desktop Stack
- **GTK4** - Native Linux desktop integration
- **libadwaita** - GNOME design system
- **Composite Templates** - UI rendering

### 🌐 Dioxus Multi-Platform Stack  
- **Dioxus** - React-like components in Rust
- **Desktop Mode** - Native windows via WebView
- **Web Mode** - Web application
- **WASM** - Browser deployment

### 🔧 System Integration
- **sysinfo** - Cross-platform system metrics
- **notify** - File system watching
- **which** - Executable discovery

## 🗂️ Project Structure

```
quickemu-manager/
├── 🦀 core/                    # Shared Rust core library
│   ├── models/                 # VM data structures  
│   └── services/               # Business logic
├── 🖥️ gtk4-app/               # Native GTK4 desktop app
│   ├── resources/ui/           # GTK4 templates
│   └── src/ui/                 # Desktop UI components
├── 🌐 dioxus-app/             # Multi-platform application
│   ├── src/components/         # Reusable UI components
│   └── src/services/           # Platform adapters
└── 📋 Cargo.toml              # Workspace configuration
```


## 🤝 Contributing

Contributions are welcome! Areas for contribution include:
- 🎨 UI/UX improvements
- 🐛 Bug fixes
- 📱 Platform-specific optimizations
- 📚 Documentation
- 🧪 Testing

## 📜 License

GPLv3 License 🎉

---

**Made with 🦀 Rust and ❤️ for the VM community**