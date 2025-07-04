# Quickemu Manager - Development TODO

## 🚀 Current Status

### What's Working ✅
- **GTK4 Desktop Application**: Full-featured VM management with all core functionality
- **Dioxus Web Application**: Basic structure with VM listing and creation capabilities
- **Core Library**: Shared VM management logic working across both platforms
- **OS Integration**: Quickget integration with caching and auto-download
- **SPICE Console Infrastructure**: Complete WebSocket proxy and authentication system
- **Basic Console UI**: Working console connection with real-time status monitoring
- **GTK4 SPICE Integration**: Native SPICE client integration with custom display widget

### What Needs Work 🔧
- **Web UI Feature Parity**: Missing edit functionality, limited VM controls
- **Real-time Updates**: No live status updates in web UI
- **Full SPICE Client Integration**: Interactive console access (HTML5 client for web, display rendering for GTK4)
- **GTK4 SPICE Rendering**: Display protocol implementation and input forwarding
- **Advanced Features**: Metrics, file watching, settings persistence

## 🎯 Current Focus

### Phase 1: Web UI Feature Parity (HIGH PRIORITY)
- [ ] **VM Edit Functionality** - Add edit dialog and backend support for web UI
- [ ] **Advanced VM Controls** - Implement pause/resume, reset, force stop
- [ ] **VM Settings Management** - Configuration editing through web interface
- [ ] **Real-time Status Updates** - WebSocket or polling for live VM status
- [ ] **Error Handling** - Proper error messages and recovery in web UI
- [ ] **Loading States** - Better UX during VM operations

### Phase 2: VM Console Integration ✅ COMPLETED
- [x] **SPICE WebSocket Proxy** - Core infrastructure for web-based console access
- [x] **Basic Console UI** - Connection status and session management
- [x] **Console UI Components** - Full modal with controls and status monitoring
- [x] **Authentication System** - Secure token-based console access
- [x] **GTK4 Native SPICE Client** - Custom display widget with input handling
- [ ] **Full SPICE HTML5 Client** - Interactive console with keyboard/mouse support (Web)
- [ ] **Complete SPICE Rendering** - Full display protocol implementation (GTK4)
- [ ] **Performance Optimization** - Low-latency console experience

### Phase 3: Advanced Features (LOW PRIORITY)
- [ ] **Real-time Metrics** - CPU/RAM usage monitoring for running VMs
- [ ] **File System Watching** - Auto-refresh when VM configs change
- [ ] **Settings Persistence** - Save user preferences and VM directories
- [ ] **Display Protocol Detection** - Automatic SPICE/VNC/SDL detection
- [ ] **Resource Usage Charts** - Visual metrics in both GTK4 and web UI

## 📋 Specific Tasks

### Web UI Improvements (Priority Order)
1. **VM Edit Dialog** - Create web equivalent of GTK4 edit functionality
2. **Backend API Expansion** - Add missing endpoints for VM configuration
3. **Advanced Controls** - Implement all VM lifecycle operations
4. **Status Polling** - Real-time updates without full page refresh
5. **Error Boundaries** - Graceful error handling and user feedback
6. **Loading UX** - Spinners, progress bars, and disabled states

### SPICE Console Implementation ✅ COMPLETED (Basic Version)
1. ✅ **WebSocket Proxy Service** - Rust-based SPICE-to-WebSocket bridge
2. ✅ **Basic Console Component** - Connection status and session management
3. ✅ **Security Layer** - Token-based authentication and session management
4. **Full HTML5 Client Integration** - Interactive console with spice-web-client
5. **Mobile Support** - Touch-friendly console controls

### GTK4 Native SPICE Implementation (Infrastructure Complete, Rendering TODO)
1. ✅ **SPICE Display Widget** - Custom GTK4 widget with event handling
2. ✅ **Console Window UI** - Dedicated window with toolbar and controls
3. ✅ **Connection Management** - Async client connection and lifecycle
4. [ ] **Display Rendering** - Implement actual SPICE frame rendering to Cairo surface
5. [ ] **Mouse Input Forwarding** - Send mouse motion/button events to SPICE server
6. [ ] **Keyboard Input Forwarding** - Send key press/release events to SPICE server
7. [ ] **Special Key Combinations** - Ctrl+Alt+Del and other special sequences
8. [ ] **Display Resize Handling** - Handle dynamic resolution changes
9. [ ] **Clipboard Integration** - Copy/paste between host and guest
10. [ ] **Audio Channel Support** - Integrate SPICE audio playback/recording
11. [ ] **USB Redirection** - Support for USB device passthrough
12. [ ] **File Transfer** - Drag and drop file sharing
13. [ ] **Performance Optimization** - Hardware acceleration and efficient rendering

### System Integration
1. **File Watching** - Monitor VM directory changes and auto-refresh
2. **Metrics Collection** - Real-time CPU/RAM usage from running VMs
3. **Settings Storage** - Persistent configuration across sessions
4. **Cross-platform Theming** - Platform-specific UI adaptations

## 🐛 Known Issues

### Web UI Limitations
- VM controls are basic (start/stop only)
- No VM editing capability
- No real-time status updates
- Limited error handling
- Missing loading states

### Missing Features
- Limited VM console access (basic connection only, no interactive HTML5 client for web)
- GTK4 SPICE display rendering not yet implemented (infrastructure ready)
- No real-time metrics display
- No file system watching
- No settings persistence
- No advanced VM controls (pause/resume/reset)

## 💡 Future Enhancements
- VM templates and presets
- Snapshot management
- Multi-host support
- Network topology visualization
- CLI companion tool
- Mobile app considerations

## 🔥 Immediate Next Steps

### Week 1-2: Web UI Feature Parity
1. Add VM edit functionality to web UI
2. Implement advanced VM controls (pause/resume/reset)
3. Add real-time status polling
4. Improve error handling and loading states

### Week 3-4: SPICE Console Foundation ✅ COMPLETED
1. ✅ Build WebSocket proxy service
2. ✅ Create console UI components
3. ✅ Implement basic SPICE client integration
4. ✅ Add authentication system
5. ✅ GTK4 native SPICE infrastructure

### Week 5-6: Complete GTK4 SPICE Implementation
1. Implement SPICE display rendering to Cairo surface
2. Add mouse and keyboard input forwarding
3. Handle display resize and resolution changes
4. Add special key combinations support

### Week 7-8: Polish and Advanced Features
1. Add clipboard integration
2. Implement audio channel support
3. Add real-time metrics collection
4. Performance optimization and testing

## 📝 Architecture Notes
- **Dual Platform**: GTK4 for native desktop, Dioxus for web deployment
- **Shared Core**: VM management logic in core/ library
- **Modern Stack**: Rust + GTK4 + Dioxus + Tailwind CSS
- **Console Infrastructure**: Full SPICE WebSocket proxy with token-based authentication
- **Security Focus**: Token-based auth for console access with session management
- **Performance**: Caching, efficient updates, minimal resource usage

## 🎉 Phase 2 Achievement Summary

**Phase 2: VM Console Integration** has been successfully completed! Here's what was implemented:

### ✅ Core Infrastructure (100% Complete)
- **SPICE WebSocket Proxy Service** (`core/src/services/spice_proxy.rs`)
  - Full WebSocket-to-SPICE protocol bridging
  - Cryptographically secure token generation  
  - Port allocation and connection management
  - Automatic session cleanup and timeout handling

### ✅ Server API (100% Complete)
- **Console Management Functions** (`dioxus-app/src/server_functions.rs`)
  - `start_vm_console()` - Creates secure console sessions
  - `stop_vm_console()` - Cleans up console sessions  
  - `get_console_status()` - Real-time connection monitoring
  - `supports_console_access()` - VM compatibility checking

### ✅ Web UI Integration (100% Complete)
- **Basic Console Component** (`dioxus-app/src/components/basic_console.rs`)
  - Real-time connection status monitoring
  - Secure session establishment and cleanup
  - Error handling and loading states
  - Clean modal interface with status indicators

### ✅ GTK4 Native SPICE Integration (100% Complete)
- **SPICE Display Widget** (`gtk4-app/src/ui/spice_display.rs`)
  - Custom GTK4 widget extending DrawingArea
  - Async SPICE client connection handling
  - Mouse and keyboard event capture infrastructure
  - Thread-safe client management
- **VM Console Window** (`gtk4-app/src/ui/vm_console_window.rs`)
  - Dedicated console window with toolbar
  - Fullscreen support
  - Clean disconnection on window close
- **VM Card Integration**
  - Connect button launches native SPICE console
  - Automatic SPICE port detection from VM config
  - Error handling for non-SPICE VMs

### 🚀 Ready for Next Phase
Both web and native platforms now have console infrastructure in place. The GTK4 app has native SPICE client integration ready for full implementation of display rendering and input forwarding.