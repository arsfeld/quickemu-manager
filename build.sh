#!/bin/bash

# Build script for Quickemu Manager

# Add Rust to PATH
export PATH="/home/linuxbrew/.linuxbrew/opt/rustup/bin:$PATH"

# Find and set PKG_CONFIG_PATH
if command -v pkg-config &> /dev/null; then
    # Add homebrew paths
    export PKG_CONFIG_PATH="/home/linuxbrew/.linuxbrew/lib/pkgconfig:/home/linuxbrew/.linuxbrew/share/pkgconfig:$PKG_CONFIG_PATH"
    
    # Add specific package paths that might be needed
    for pkg in xorgproto libx11 libxext libxrender libxau libxdmcp cairo pango gdk-pixbuf gtk+3 webkitgtk; do
        # Use shell expansion to resolve wildcards
        for pkg_path in /home/linuxbrew/.linuxbrew/Cellar/$pkg/*/lib/pkgconfig; do
            if [ -d "$pkg_path" ]; then
                export PKG_CONFIG_PATH="$pkg_path:$PKG_CONFIG_PATH"
            fi
        done
        
        for share_path in /home/linuxbrew/.linuxbrew/Cellar/$pkg/*/share/pkgconfig; do
            if [ -d "$share_path" ]; then
                export PKG_CONFIG_PATH="$share_path:$PKG_CONFIG_PATH"
            fi
        done
    done
fi

echo "Building Quickemu Manager..."
echo "PKG_CONFIG_PATH: $PKG_CONFIG_PATH"

# Add system library paths for xdo
export LIBRARY_PATH="/usr/lib64:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/usr/lib64:$LD_LIBRARY_PATH"
export RUSTFLAGS="-L /usr/lib64 $RUSTFLAGS"

echo "LIBRARY_PATH: $LIBRARY_PATH"
echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"
echo "RUSTFLAGS: $RUSTFLAGS"

cargo build --release

if [ $? -eq 0 ]; then
    echo "Build successful! Binary located at: target/release/quickemu-manager"
else
    echo "Build failed. You may need to install GTK development libraries:"
    echo "  - On Fedora: sudo dnf install gtk3-devel webkit2gtk4.1-devel"
    echo "  - On Ubuntu: sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev"
    echo "  - On macOS: brew install gtk+3 webkit2gtk"
fi