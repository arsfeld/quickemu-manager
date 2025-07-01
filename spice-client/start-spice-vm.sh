#!/bin/bash
# Simple QEMU VM with SPICE server for testing

# Download a small test image if needed (Alpine Linux - very small)
if [ ! -f alpine-virt-3.19.0-x86_64.iso ]; then
    echo "Downloading Alpine Linux ISO for testing..."
    wget https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-virt-3.19.0-x86_64.iso
fi

# Start QEMU with SPICE
echo "Starting QEMU with SPICE on port 5900..."
qemu-system-x86_64 \
    -name "SPICE Test VM" \
    -machine q35 \
    -cpu host \
    -enable-kvm \
    -m 512 \
    -boot d \
    -cdrom alpine-virt-3.19.0-x86_64.iso \
    -spice port=5900,addr=127.0.0.1,disable-ticketing=on \
    -vga qxl \
    -monitor stdio

# Notes:
# - SPICE server will listen on localhost:5900
# - No password/ticketing is required (disable-ticketing=on)
# - The VM display is only available via SPICE
# - Use Ctrl+C in the terminal to stop the VM