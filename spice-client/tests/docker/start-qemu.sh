#!/bin/bash

echo "Starting QEMU with SPICE server..."

# Start QEMU with minimal configuration
# - No KVM (for container compatibility)
# - SPICE on port 5900 without authentication
# - Basic VGA display
# - Minimal memory
exec qemu-system-x86_64 \
    -machine pc \
    -m 128 \
    -nodefaults \
    -nographic \
    -serial none \
    -monitor none \
    -spice port=5900,addr=0.0.0.0,disable-ticketing=on,image-compression=off \
    -vga qxl \
    -device virtio-serial \
    -chardev spicevmc,id=vdagent,debug=0,name=vdagent \
    -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 \
    -hda /tmp/test.qcow2