#!/bin/bash
# Build script for all platforms

set -e

echo "Building Quickemu Manager for all platforms..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Platforms
LINUX_TARGET="x86_64-unknown-linux-gnu"
MACOS_INTEL_TARGET="x86_64-apple-darwin"
MACOS_ARM_TARGET="aarch64-apple-darwin"

# Build spice-client
echo -e "${BLUE}Building spice-client...${NC}"
cd spice-client

echo "  Building for Linux..."
cargo build --release --target $LINUX_TARGET
cargo build --release --target $LINUX_TARGET --features backend-gtk4

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "  Building for macOS..."
    cargo build --release --target $MACOS_INTEL_TARGET
    cargo build --release --target $MACOS_ARM_TARGET
fi

echo "  Building WASM..."
if ! command -v wasm-pack &> /dev/null; then
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi
wasm-pack build --target web --out-dir pkg

cd ..

# Build GTK4 app (Linux and macOS only)
    echo -e "${BLUE}Building GTK4 app...${NC}"
    cd gtk4-app
    
    echo "  Building for Linux..."
    cargo build --release --target $LINUX_TARGET
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "  Building for macOS..."
        cargo build --release --target $MACOS_INTEL_TARGET
        cargo build --release --target $MACOS_ARM_TARGET
    fi
    
    cd ..

# Build Dioxus app
echo -e "${BLUE}Building Dioxus app...${NC}"
cd dioxus-app

if ! command -v dx &> /dev/null; then
    echo "Installing Dioxus CLI..."
    cargo install dioxus-cli
fi

echo "  Building desktop app for Linux..."
dx build --release --platform desktop --target $LINUX_TARGET

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "  Building desktop app for macOS..."
    dx build --release --platform desktop --target $MACOS_INTEL_TARGET
    dx build --release --platform desktop --target $MACOS_ARM_TARGET
fi

echo "  Building web app..."
dx build --release --platform web

cd ..

echo -e "${GREEN}Build completed successfully!${NC}"
echo ""
echo "Build artifacts:"
echo "  - spice-client: spice-client/target/*/release/"
echo "  - GTK4 app: gtk4-app/target/*/release/"
echo "  - Dioxus app: dioxus-app/dist/"
echo "  - WASM: spice-client/pkg/"