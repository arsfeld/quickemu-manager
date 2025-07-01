#!/bin/bash
set -e

echo "Building GTK4 client..."
cargo build --bin rusty-spice-gtk --features backend-gtk4

echo "Starting test QEMU VM..."
docker-compose -f docker/docker-compose.test.yml up -d test-vm

echo "Waiting for VM to be ready..."
sleep 5

echo "Running GTK4 client..."
RUST_LOG=debug,spice_client=info cargo run --bin rusty-spice-gtk --features backend-gtk4 -- -H localhost -p 5900 -d

echo "Test complete. Press Ctrl+C to stop the VM."
wait