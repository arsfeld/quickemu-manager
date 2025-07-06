# Slint Frontend TODO

This document tracks the remaining work needed to achieve feature parity with the GTK4 frontend.

## Core Features

### VM Management
- [ ] VM console view with SPICE display integration
  - [ ] Integrate spice-client for console display
  - [ ] Handle console connection/disconnection
  - [ ] Show console in main window with back navigation
- [ ] VM context menu (right-click)
  - [ ] Edit VM option
  - [ ] Delete VM with confirmation dialog
  - [ ] Clone VM functionality
  - [ ] Export VM functionality
- [ ] VM creation dialog
  - [ ] Template selection (OS list)
  - [ ] VM configuration options (CPU, RAM, disk size)
  - [ ] Progress tracking during VM creation
  - [ ] Integration with quickget for downloading ISOs

### UI Components
- [ ] Settings dialog
  - [ ] VM directories configuration
  - [ ] Theme preferences (dark/light mode)
  - [ ] Default VM settings
- [ ] About dialog
  - [ ] Application version and credits
  - [ ] License information
  - [ ] Links to documentation/GitHub
- [ ] Toast notifications
  - [ ] Success/error messages
  - [ ] VM state change notifications
  - [ ] Auto-dismiss after timeout
- [ ] Empty state improvements
  - [ ] Better messaging when no VMs found
  - [ ] Quick action to create first VM

### VM Display Enhancements
- [ ] VM status icons
  - [ ] Different icons for each status (running, stopped, error, etc.)
  - [ ] Animated icons for transitional states (starting, stopping)
- [ ] Live metrics display
  - [ ] CPU usage percentage with ProcessMonitor integration
  - [ ] Memory usage with proper formatting
  - [ ] Disk I/O statistics
- [ ] VM type icons
  - [ ] OS-specific icons (Ubuntu, Fedora, Windows, etc.)
  - [ ] Custom icon support from VM config

### Layout & Responsive Design
- [ ] Grid layout for VM cards
  - [ ] Responsive columns based on window width
  - [ ] Proper spacing and alignment
  - [ ] Smooth transitions on layout changes
- [ ] Window state persistence
  - [ ] Remember window size and position
  - [ ] Save/restore view state

### Keyboard Shortcuts
- [ ] Global shortcuts
  - [ ] Ctrl+N: Create new VM
  - [ ] Ctrl+R: Refresh VM list
  - [ ] Ctrl+Q: Quit application
  - [ ] F11: Toggle fullscreen
- [ ] VM-specific shortcuts
  - [ ] Enter: Start/open console for selected VM
  - [ ] Delete: Remove selected VM
  - [ ] Space: Toggle VM state (start/stop)

### Advanced Features
- [ ] VM search/filter
  - [ ] Search by name
  - [ ] Filter by status
  - [ ] Filter by OS type
- [ ] Bulk operations
  - [ ] Select multiple VMs
  - [ ] Start/stop multiple VMs
  - [ ] Delete multiple VMs
- [ ] VM templates
  - [ ] Save VM as template
  - [ ] Create VM from custom template
- [ ] Import/Export
  - [ ] Import existing VMs from directory
  - [ ] Export VM configuration and disk

### Platform Integration
- [ ] System tray support
  - [ ] Minimize to tray option
  - [ ] Quick VM controls from tray
  - [ ] Notifications via system tray
- [ ] File associations
  - [ ] Open .conf files with the app
  - [ ] Drag & drop VM configs
- [ ] Desktop integration
  - [ ] Proper app icon
  - [ ] Desktop file for Linux
  - [ ] Windows installer/portable package
  - [ ] macOS app bundle

### Performance & Polish
- [ ] Optimize VM status updates
  - [ ] Batch updates to reduce UI redraws
  - [ ] Debounce rapid status changes
  - [ ] Background thread for status polling
- [ ] Smooth animations
  - [ ] Card hover effects
  - [ ] State transition animations
  - [ ] Loading states for async operations
- [ ] Error handling
  - [ ] Graceful error messages
  - [ ] Recovery options for common errors
  - [ ] Detailed error logs

### Testing & Documentation
- [ ] Unit tests for UI logic
- [ ] Integration tests with mock VMs
- [ ] User documentation
  - [ ] Getting started guide
  - [ ] Feature documentation
  - [ ] Troubleshooting guide
- [ ] Developer documentation
  - [ ] Architecture overview
  - [ ] Contributing guide
  - [ ] API documentation

## Build & Distribution
- [ ] Add to build-all.sh script
- [ ] Include in CI/CD pipelines
- [ ] Package for distribution
  - [ ] AppImage for Linux
  - [ ] MSI installer for Windows
  - [ ] DMG for macOS
- [ ] Add to release workflow

## Nice-to-Have Features
- [ ] VM snapshots UI
- [ ] Network configuration UI
- [ ] USB device passthrough UI
- [ ] Shared folders configuration
- [ ] VNC support (in addition to SPICE)
- [ ] Remote VM management
- [ ] VM performance graphs/history
- [ ] Backup/restore functionality

## Technical Debt
- [ ] Improve error handling throughout
- [ ] Add proper logging with different levels
- [ ] Implement proper state management pattern
- [ ] Add accessibility features
- [ ] Internationalization (i18n) support
- [ ] Theme customization API

## Known Issues
- [ ] Grid layout not available in Slint (using VerticalLayout instead)
- [ ] Platform-specific features may need conditional compilation
- [ ] SPICE integration needs careful handling of widget lifecycle
- [ ] File system watching may have platform-specific quirks

## Priority Order
1. VM console view (critical for usability)
2. VM creation dialog (essential feature)
3. Context menu operations (edit, delete, clone)
4. Settings and About dialogs
5. Toast notifications
6. Live metrics display
7. Grid layout improvements
8. Keyboard shortcuts
9. Platform integration
10. Advanced features