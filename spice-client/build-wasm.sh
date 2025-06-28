#!/bin/bash
# Build script for WASM version of spice-client

set -e

echo "Building WASM package..."

# Ensure wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build the WASM package
wasm-pack build --target web --out-dir pkg

echo "WASM build complete!"
echo "Output files are in the pkg/ directory"