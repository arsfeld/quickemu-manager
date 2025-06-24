# Quickemu Manager

A modern, cross-platform GUI application for managing Quickemu virtual machines built with Rust and Dioxus.

## Features

- 🚀 **Native Performance**: Built with Rust for speed and efficiency
- 🖥️ **Cross-Platform**: Works on Linux (GNOME) and macOS
- 📊 **Real-time Monitoring**: CPU and RAM usage metrics for each VM
- 🔍 **Auto-Discovery**: Automatically finds and monitors VM configurations
- 🎨 **Modern UI**: Clean, minimalistic interface with dark mode support
- 📦 **Single Binary**: Distributed as a single executable file

## Prerequisites

- Rust 1.75 or later
- [quickemu](https://github.com/quickemu-project/quickemu) installed
- [quickget](https://github.com/quickemu-project/quickget) (optional, for VM creation)

### System Dependencies (for GUI version)

**Fedora/RHEL:**
```bash
sudo dnf install gtk3-devel webkit2gtk4.1-devel cairo-devel pango-devel
```

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev
```

**macOS:**
```bash
brew install gtk+3 webkit2gtk
```

**Arch Linux:**
```bash
sudo pacman -S gtk3 webkit2gtk-4.1
```

## Building

```bash
# Clone the repository
git clone https://github.com/yourusername/quickemu-manager.git
cd quickemu-manager

# Option 1: Build with GUI (requires GTK dependencies)
./build.sh

# Option 2: Build with cargo directly
export PATH="/home/linuxbrew/.linuxbrew/opt/rustup/bin:$PATH"  # If using Homebrew Rust
cargo build --release

# Run the application
cargo run --release

# Or run the built binary
./target/release/quickemu-manager
```

### Troubleshooting Build Issues

If you encounter build errors related to GTK or WebKit:

1. **Check pkg-config**: Ensure pkg-config can find the libraries:
   ```bash
   pkg-config --libs gtk+-3.0
   ```

2. **Set PKG_CONFIG_PATH**: If libraries are installed in non-standard locations:
   ```bash
   export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH"
   ```

3. **Alternative TUI version**: A terminal UI version is planned that doesn't require GTK.

## Usage

1. **First Launch**: The app will scan default directories for existing VMs:
   - `~/.config/quickemu/`
   - `~/VMs/`

2. **Managing VMs**: 
   - Click on a VM card to see details
   - Use the Start/Stop buttons to control VMs
   - Click "Open Console" to launch the VM display

3. **Creating VMs**: (Coming soon)
   - Click "Create VM" button
   - Select OS and version
   - Configure resources

## Configuration

Settings are stored in:
- Linux: `~/.config/quickemu-manager/config.toml`
- macOS: `~/Library/Application Support/quickemu-manager/config.toml`

## Development

This project uses:
- **Dioxus**: Reactive UI framework
- **Tokio**: Async runtime
- **Serde**: Serialization
- **Notify**: File watching
- **Sysinfo**: System metrics

## Project Structure

```
src/
├── models/       # Data structures
├── services/     # Backend services
├── components/   # UI components
└── main.rs       # Application entry point
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.