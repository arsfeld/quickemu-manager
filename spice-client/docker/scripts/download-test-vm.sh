#!/bin/bash
set -e

# Download a minimal test VM if not already present
VM_PATH="/data/test-vm.qcow2"

if [ ! -f "$VM_PATH" ]; then
    echo "Downloading test VM image..."
    
    # Option 1: Use a minimal cirros image for testing
    curl -L -o "$VM_PATH" \
        https://download.cirros-cloud.net/0.6.2/cirros-0.6.2-x86_64-disk.img
    
    # Option 2: Create a small empty disk for basic testing
    # qemu-img create -f qcow2 "$VM_PATH" 1G
    
    echo "Test VM image ready at $VM_PATH"
else
    echo "Test VM image already exists at $VM_PATH"
fi

# Create a marker file to indicate download is complete
touch /data/.vm-ready