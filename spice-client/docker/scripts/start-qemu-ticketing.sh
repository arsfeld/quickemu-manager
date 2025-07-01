#!/bin/bash
set -e

# Wait for VM image to be ready
while [ ! -f /data/.vm-ready ]; do
    echo "Waiting for VM image to be downloaded..."
    sleep 2
done

VM_PATH="/data/test-vm.qcow2"
SPICE_PORT="${SPICE_PORT:-5900}"
MONITOR_PORT="${MONITOR_PORT:-5555}"
VNC_PORT="${VNC_PORT:-5901}"

echo "Starting QEMU with SPICE server on port $SPICE_PORT (with ticketing enabled)..."

# Start QEMU with SPICE support - WITHOUT disable-ticketing
exec qemu-system-x86_64 \
    -name "spice-test-vm" \
    -machine pc,accel=tcg \
    -cpu max \
    -m 256 \
    -drive file="$VM_PATH",format=qcow2,if=virtio \
    -spice port=$SPICE_PORT,addr=0.0.0.0,password=test123,image-compression=off,streaming-video=all \
    -device virtio-serial-pci \
    -device virtserialport,chardev=spicechannel0,name=com.redhat.spice.0 \
    -chardev spicevmc,id=spicechannel0,name=vdagent \
    -device qxl-vga,ram_size=67108864,vram_size=67108864,vgamem_mb=16 \
    -vnc :1 \
    -monitor tcp:0.0.0.0:$MONITOR_PORT,server,nowait \
    -nographic