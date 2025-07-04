# Quickemu Manager - Product Guidelines

## Product Vision
A modern, minimalistic virtual machine management application that simplifies the creation and management of quickemu VMs through an intuitive cross-platform interface.

## Core Principles
1. **Simplicity First**: Complex VM management made simple
2. **Native Integration**: Feels at home on both GNOME and macOS
3. **Zero Configuration**: Works out of the box with sensible defaults
4. **Single Binary**: One file to rule them all - no complex installations

## Target Users
- **Primary**: Developers and power users who need quick VM access
- **Secondary**: System administrators managing multiple VMs
- **Tertiary**: Students and educators using VMs for learning

## Key Features

### MVP (Version 1.0)
- [ ] VM discovery and listing from `.conf` files
- [ ] Create new VMs via quickget integration
- [ ] Start/stop/restart VM controls
- [ ] Real-time CPU/RAM monitoring per VM
- [ ] Native display launching (SPICE/VNC/SDL)
- [ ] Auto-scan configured directories
- [ ] Basic VM state management

### Future Considerations (Post-MVP)
- [ ] WebUI mode for remote management
- [ ] Snapshot management
- [ ] VM templates and presets
- [ ] Batch operations
- [ ] Resource limits and quotas
- [ ] VM migration between hosts

## User Experience Guidelines

### Design Philosophy
- **Minimal Chrome**: Focus on content, not UI elements
- **Information Hierarchy**: Most important info (VM state) always visible
- **Progressive Disclosure**: Advanced features hidden until needed
- **Responsive Feedback**: Every action has immediate visual feedback

### UI Patterns
- Card-based VM display with live metrics
- Floating action button for VM creation
- Contextual menus for VM operations
- Toast notifications for background operations
- Modal dialogs only for destructive actions

### Platform Integration
- **Linux/GNOME**: Follow Adwaita design patterns
- **macOS**: Respect system accent colors and themes
- **Dark Mode**: First-class support with proper contrast ratios

## Functional Requirements

### VM Management
1. **Discovery**: Auto-scan user-defined directories for `.conf` files
2. **Creation**: Streamlined quickget integration with OS selection
3. **Control**: Start, stop, pause, restart operations
4. **Monitoring**: Real-time resource usage visualization
5. **Configuration**: Edit VM settings through native `.conf` format

### System Integration
1. **Process Management**: Handle quickemu processes lifecycle
2. **File Watching**: React to external `.conf` changes
3. **Resource Monitoring**: System-level CPU/RAM tracking
4. **Display Protocol**: Launch appropriate viewer (SPICE/VNC)

### Data Management
1. **Configuration**: Store app settings in platform-appropriate location
2. **VM Registry**: Track VMs across multiple directories
3. **State Persistence**: Remember window size, selected VMs, etc.

## Non-Functional Requirements

### Performance
- Startup time: < 1 second
- VM list refresh: < 100ms
- Resource monitoring: 1Hz update rate
- Memory usage: < 50MB baseline

### Reliability
- Graceful handling of quickemu failures
- No data loss on crash
- Automatic recovery of VM states

### Security
- No elevated privileges unless required
- Secure handling of VM passwords/keys
- Sandboxed file system access

## Success Metrics
1. **Time to first VM**: < 2 minutes from app launch
2. **User satisfaction**: Minimal support requests
3. **Cross-platform parity**: 95% feature compatibility
4. **Binary size**: < 30MB compressed

## Out of Scope
- VM disk image management
- Network configuration beyond quickemu defaults
- Backup/restore functionality
- Multi-user access control
- Cloud provider integration