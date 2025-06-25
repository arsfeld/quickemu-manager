# Checklist: Converting `quickemu` Bash Script to Rust Standalone

This document outlines the comprehensive steps needed to convert the 2200+ line `quickemu` bash script into a structured Rust standalone application.

## 1. **Project Structure Setup**
- [ ] Create a new Rust project with `cargo new quickemu_rs --bin`
- [ ] Add necessary dependencies to `Cargo.toml`:
  - [ ] `clap` for command-line argument parsing
  - [ ] `serde` and `serde_derive` for configuration file parsing
  - [ ] `tokio` for async operations
  - [ ] `anyhow` or `thiserror` for error handling
  - [ ] `log` and `env_logger` for logging
  - [ ] `regex` for string pattern matching
  - [ ] `which` for finding executables
  - [ ] `nix` for Unix system calls
  - [ ] `dirs` for home directory access

## 2. **Core Data Structures**
- [ ] Define `VmConfig` struct to represent VM configuration
- [ ] Define `VmState` struct to track running VM state
- [ ] Define enums for:
  - [ ] `GuestOs` (windows, macos, linux, etc.)
  - [ ] `DisplayType` (gtk, spice, vnc, etc.)
  - [ ] `SoundCard` types
  - [ ] `Monitor` types (socket, telnet, none)

## 3. **Configuration Management**
- [ ] Implement configuration file parser (`.conf` files)
- [ ] Create default configuration generator
- [ ] Implement validation for configuration parameters
- [ ] Handle environment variable expansion

## 4. **VM Lifecycle Management Functions**
- [ ] `vm_boot()` - Main VM starting logic
- [ ] `kill_vm()` - Stop running VM
- [ ] `delete_vm()` - Complete VM removal
- [ ] `delete_disk()` - Disk image deletion
- [ ] Process ID management (reading/writing `.pid` files)

## 5. **Snapshot Management**
- [ ] `snapshot_create()` - Create VM snapshots
- [ ] `snapshot_apply()` - Apply existing snapshots
- [ ] `snapshot_delete()` - Remove snapshots
- [ ] `snapshot_info()` - Display snapshot information
- [ ] Integration with `qemu-img` command

## 6. **Hardware Configuration**
- [ ] CPU configuration based on guest OS and host capabilities
- [ ] Memory allocation logic
- [ ] USB device passthrough configuration
- [ ] Sound system setup
- [ ] Network interface configuration
- [ ] Display and graphics setup

## 7. **Guest OS Specific Logic**
- [ ] macOS-specific configurations (SMC, CPU flags, bootloader)
- [ ] Windows-specific settings (Hyper-V enlightenments, SecureBoot)
- [ ] Linux distribution optimizations
- [ ] Special handling for: batocera, freedos, haiku, solaris, kolibrios, reactos

## 8. **System Integration**
- [ ] Host system detection (Linux, macOS, Windows)
- [ ] CPU feature detection and validation
- [ ] KVM configuration and MSR handling
- [ ] TPM (Trusted Platform Module) support
- [ ] UEFI/BIOS firmware management

## 9. **External Tool Integration**
- [ ] QEMU command building and execution
- [ ] `qemu-img` operations wrapper
- [ ] Viewer application launching (virt-viewer, spicy, etc.)
- [ ] Terminal monitoring socket communication

## 10. **Command Line Interface**
- [ ] Argument parsing for all current CLI options:
  - [ ] VM configuration file path
  - [ ] Snapshot operations (`--snapshot create/apply/delete/info`)
  - [ ] VM actions (`--kill`, `--delete-disk`, `--delete-vm`)
  - [ ] Display options (`--display`, `--viewer`)
  - [ ] Hardware options (`--cpu-cores`, `--ram`, `--sound-card`)
  - [ ] Monitor options (`--monitor`, `--monitor-telnet-host/port`)
- [ ] Usage/help text generation

## 11. **Port Management**
- [ ] Dynamic port allocation for services
- [ ] Port conflict detection and resolution
- [ ] Port file management (`.ports` files)

## 12. **File and Directory Operations**
- [ ] VM directory structure creation/management
- [ ] Disk image file handling
- [ ] ISO/image file management
- [ ] Desktop shortcut creation/deletion

## 13. **Error Handling and Validation**
- [ ] Input validation for all parameters
- [ ] Graceful error handling with meaningful messages
- [ ] Dependency checking (QEMU, required files)
- [ ] Permission validation

## 14. **Monitoring and Communication**
- [ ] Monitor socket communication
- [ ] Serial socket handling
- [ ] Command sending to running VMs
- [ ] Process status monitoring

## 15. **Special Features**
- [ ] Unattended Windows installation support
- [ ] File sharing between host and guest (9p)
- [ ] SPICE/VNC integration
- [ ] Screen recording capabilities

## 16. **Testing and Documentation**
- [ ] Unit tests for core functions
- [ ] Integration tests for VM operations
- [ ] Documentation for public APIs
- [ ] Usage examples and README

## 17. **Cross-Platform Considerations**
- [ ] Handle differences between Linux, macOS, and Windows hosts
- [ ] Path separator handling
- [ ] Process management differences
- [ ] File permission handling

## 18. **Performance Optimizations**
- [ ] Async operations where appropriate
- [ ] Efficient string handling
- [ ] Minimal memory allocations
- [ ] Fast startup times

## Implementation Notes

### Key Functions to Convert

The original bash script contains several critical functions that need Rust equivalents:

- **VM Management**: `vm_boot()`, `kill_vm()`, `delete_vm()`, `delete_disk()`
- **Snapshots**: `snapshot_create()`, `snapshot_apply()`, `snapshot_delete()`, `snapshot_info()`
- **Hardware**: `configure_usb()`, CPU configuration logic, memory setup
- **System**: `ignore_msrs_always()`, `ignore_msrs_alert()`
- **Utilities**: `get_port()`, port management, file operations

### Configuration File Format

The script reads `.conf` files with shell variable syntax. The Rust version should:
- Parse key=value pairs
- Handle quoted strings and environment variable expansion
- Validate configuration parameters
- Provide helpful error messages for invalid configs

### QEMU Integration

The script builds complex QEMU command lines. The Rust version needs:
- A builder pattern for QEMU arguments
- Type-safe parameter validation
- Guest OS-specific argument generation
- Process management for QEMU instances

### Compatibility Requirements

- Must support all current command-line options
- Should maintain compatibility with existing `.conf` files
- Must handle all supported guest operating systems
- Should preserve current behavior for edge cases

---

*Generated on: June 24, 2025*
*Source: Analysis of quickemu bash script (2223 lines)*
