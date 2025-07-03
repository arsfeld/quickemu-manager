#!/bin/bash

echo "Starting minimal SPICE test server..."

# Start QEMU with test card display
# This provides a simple test pattern without needing a full OS
exec qemu-system-x86_64 \
    -machine none \
    -device VGA \
    -spice port=5900,addr=0.0.0.0,disable-ticketing=on,image-compression=off \
    -display none