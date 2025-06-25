# ⚡ Quickemu Manager

> **The blazingly fast, multi-platform VM management experience you've been waiting for** 🚀

Built with **Rust** 🦀 for uncompromising performance and reliability. Experience VM management at the speed of thought with our revolutionary dual-UI architecture.

## ✨ Why Quickemu Manager?

🔥 **Blazingly Fast** - Written in Rust, startup in <500ms, native performance  
🎯 **Zero Friction** - Auto-discovery finds your VMs instantly  
🌐 **Universal Access** - Native desktop app + web interface  
📊 **Real-time Intel** - Live metrics, instant status updates  
🎨 **Beautiful UI** - Modern design that adapts to your workflow  
⚡ **Lightning Quick** - Sub-second VM operations  
🔒 **Rock Solid** - Memory-safe Rust prevents crashes  

## 🏗️ Revolutionary Architecture

**Two Independent Apps, One Powerful Core** 💪

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

### 🎪 Choose Your Experience

**🖥️ GTK4 Desktop** - Lightning-fast native Linux experience  
**🌐 Dioxus Multi-Platform** - Run anywhere: Linux, Windows, macOS, Web Browser  
**⚡ Shared Performance** - Same blazing-fast Rust core powers both  

## 🚀 Getting Started

### Prerequisites
- 🦀 Rust 1.75+ (for that sweet performance)
- ⚡ [quickemu](https://github.com/quickemu-project/quickemu) installed
- 🎯 [quickget](https://github.com/quickemu-project/quickget) (optional, for VM creation magic)

### 🏃‍♂️ Quick Install

```bash
# Clone the future of VM management
git clone https://github.com/yourusername/quickemu-manager.git
cd quickemu-manager

# 🖥️ Build GTK4 Desktop App (Native Linux Power)
cd gtk4-app
cargo build --release
cargo run

# 🌐 Build Dioxus Multi-Platform App
cd ../dioxus-app

# Desktop mode (native window, no GTK4 deps!)
cargo run --features desktop

# Web server mode (access from any browser)
cargo run --features server

# Build for deployment
cargo build --release --features desktop
```

### 🎨 Development with Hot Reload

```bash
# GTK4 app development
cd gtk4-app && cargo run

# Dioxus app with instant hot reload ⚡
cd dioxus-app
dx serve --platform desktop  # Desktop development
dx serve --platform web      # Web development

# Build everything at once 💪
cargo build --workspace
```

## 🌟 Feature Showcase

### ⚡ Performance That Matters
- **<500ms** startup time - Faster than you can blink
- **<30MB** memory footprint - Respectful of your resources  
- **<1%** CPU usage when idle - Silent when you need it
- **Real-time** updates - No delays, no waiting

### 🎯 Smart VM Discovery
Auto-finds VMs in:
- `~/.config/quickemu/`
- `~/VMs/`
- Custom directories you specify

### 📊 Live Metrics Dashboard
- 🔥 CPU usage with beautiful charts
- 💾 RAM consumption tracking  
- 💿 Disk I/O monitoring
- 🌐 Network activity
- ⏱️ Uptime tracking

### 🎮 Effortless VM Control
- ▶️ **Start/Stop** - One-click VM lifecycle management
- 🖥️ **Display Launch** - Instant console access
- ⚙️ **Live Config** - Real-time configuration viewing
- 📈 **Resource Monitoring** - Keep tabs on performance

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

### 🌐 Web App (Zero Dependencies!)
The Dioxus web app requires **no system dependencies** - just Rust! 🎉

## 🏃‍♂️ Usage

### 🎬 First Launch Experience
1. **Instant Discovery** - App scans for existing VMs automatically
2. **Beautiful Gallery** - See all your VMs in a stunning grid layout  
3. **Real-time Status** - Live indicators show VM states instantly

### 🎮 Daily Workflow
- **Click to Control** - Start/stop VMs with satisfying one-click actions
- **Live Monitoring** - Watch metrics update in real-time
- **Quick Launch** - Open VM consoles instantly
- **Smart Navigation** - Keyboard shortcuts for power users

### 🔧 Configuration
Settings auto-sync across platforms:
- **Linux**: `~/.config/quickemu-manager/config.toml`
- **macOS**: `~/Library/Application Support/quickemu-manager/config.toml`
- **Windows**: `%APPDATA%\quickemu-manager\config.toml`

## 🏗️ Tech Stack

### 🦀 Core Foundation
- **Rust** - Memory safety meets blazing performance
- **Tokio** - Async runtime for responsive UI
- **Serde** - Lightning-fast serialization

### 🖥️ GTK4 Desktop Stack
- **GTK4** - Native Linux desktop integration
- **libadwaita** - Modern GNOME design language
- **Composite Templates** - Efficient UI rendering

### 🌐 Dioxus Multi-Platform Stack  
- **Dioxus** - React-like components in Rust
- **Desktop Mode** - Native windows via WebView
- **Web Mode** - Full-stack web application
- **WASM** - Browser deployment ready

### 🔧 System Integration
- **sysinfo** - Cross-platform system metrics
- **notify** - File system watching
- **which** - Executable discovery

## 🗂️ Project Structure

```
quickemu-manager/
├── 🦀 core/                    # Shared Rust core library
│   ├── models/                 # VM data structures  
│   └── services/               # Business logic engine
├── 🖥️ gtk4-app/               # Native GTK4 desktop app
│   ├── resources/ui/           # GTK4 templates
│   └── src/ui/                 # Desktop UI components
├── 🌐 dioxus-app/             # Multi-platform application
│   ├── src/components/         # Reusable UI components
│   └── src/services/           # Platform adapters
└── 📋 Cargo.toml              # Workspace configuration
```

## 🚀 Performance Targets

| Metric | Target | Why It Matters |
|--------|---------|----------------|
| 🏃 Startup | <500ms | Instant gratification |
| 💾 Memory | <30MB base | Respectful resource usage |
| 🔥 CPU Idle | <1% | Silent operation |
| 📦 Binary Size | <20MB | Quick downloads |
| ⚡ UI Response | <16ms | Buttery smooth |

## 🤝 Contributing

We'd love your help making Quickemu Manager even more awesome! 

**Areas where you can make an impact:**
- 🎨 UI/UX improvements
- 🐛 Bug hunting and fixes  
- 📱 Platform-specific optimizations
- 📚 Documentation enhancements
- 🧪 Testing and quality assurance

## 📜 License

MIT License - Build amazing things with this code! 🎉

---

**Made with 🦀 Rust and ❤️ for the VM community**

*Experience the future of VM management today* ⚡