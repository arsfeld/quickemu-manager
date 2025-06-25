# Quickemu Manager - Development TODO

## 🎉 Development Progress

### Build Status
- ✅ **Application builds successfully** in distrobox environment
- Binary location: `target/debug/quickemu-manager`
- Build warnings present but non-critical (deprecated macros, unused imports)

## 🚀 Current Sprint

### Phase 1: Foundation ✅
- [x] Create product guidelines
- [x] Create technical briefing  
- [x] ~~Initialize Rust project with Dioxus~~ **Changed to GTK4**
- [x] Set up basic project structure with GTK4 + Rust
- [x] Implement composite template UI system
- [x] Build core UI components (main window, header bar, VM cards)
- [x] Fix signal handling and callback system
- [x] Implement quickemu config parser ✅
- [x] Create VM discovery service ✅
- [x] **✅ Successfully build the application**

### Phase 2: Core Features - **✅ COMPLETE**
- [x] **✅ Implement VM creation dialog with actual forms**
- [x] **✅ Implement settings dialog with preferences**
- [x] **✅ Add VM discovery and loading functionality**
- [x] **✅ Fix VM creation UI feedback and hanging issue**
- [x] **✅ Improve progress indication during VM creation**
- [x] **✅ Implement wizard-style VM creation with real-time output**
- [x] **✅ VM lifecycle management (start/stop/restart)**
- [x] **✅ Process monitoring integration**
- [x] **✅ Real-time VM status tracking**
- [x] **✅ VM editing functionality**
- [x] **✅ Fix VM process detection issues (container/distrobox compatibility)**
- [x] **✅ Optimize debug output and state change logging**
- [x] **✅ Button state management based on VM status**
- [x] **✅ Remove fallback OS data - use only real quickget values**
- [x] **✅ Implement quickget/quickemu availability check and auto-download**
- [x] **✅ Add edition support for complex distributions (Fedora, etc.)**
- [x] **✅ Auto-populate VM names with OS-version pattern**
- [ ] Real-time metrics collection (CPU/RAM usage)
- [ ] File watching for auto-refresh
- [ ] Settings persistence

### Phase 3: Advanced Features - **CURRENT FOCUS**
- [ ] **🎯 Real-time metrics collection (CPU/RAM usage)**
- [ ] **🎯 File watching for auto-refresh**
- [ ] **🎯 Settings persistence**
- [x] **✅ Quickget integration** - Complete with edition support and auto-download
- [ ] Display protocol launching
- [ ] Resource usage charts
- [ ] Enhanced error handling and recovery

### Phase 4: Web UI Development - **NEW PHASE**
- [ ] Add optional web server capability to GTK4 app
- [ ] Create Dioxus web frontend structure
- [ ] Implement web API for VM operations
- [ ] Build responsive web interface
- [ ] Add real-time status updates
- [ ] Test cross-platform web access

### Phase 5: Polish
- [ ] Platform-specific theming
- [ ] Performance optimization
- [ ] Binary packaging (both GTK4 and web builds)
- [ ] Documentation (including web UI setup)
- [ ] Release preparation

## 📋 Detailed Tasks

### Project Setup ✅
- [x] Create Cargo.toml with dependencies
- [x] ~~Set up Dioxus with desktop feature~~ **Using GTK4**
- [x] Configure build scripts (build.rs, resource compilation)
- [x] Add development documentation

### Core Implementation - **COMPLETE**
- [x] Basic project structure established
- [x] `services/parser.rs` - Parse quickemu .conf files ✅
- [x] `services/vm_manager.rs` - VM lifecycle management ✅
- [x] `services/discovery.rs` - Find and monitor VMs ✅
- [x] `services/process.rs` - Process monitoring ✅
- [x] `services/metrics.rs` - Resource usage tracking (skeleton)

### UI Components - **COMPLETE**
- [x] `ui/vm_card.rs` - VM display card (composite template) ✅
- [x] `ui/main_window.rs` - Main window (composite template) ✅
- [x] `ui/header_bar.rs` - Header bar (composite template) ✅
- [x] **✅ Wizard-style VM creation dialog with real-time output**
- [x] **✅ Settings dialog with preferences**
- [x] **✅ VM edit dialog for configuration changes**
- [ ] `ui/vm_detail.rs` - Detailed VM view
- [ ] Resource usage charts/graphs

### Models ✅
- [x] `models/vm.rs` - VM data structures
- [x] `models/config.rs` - App configuration
- [x] `models/metrics.rs` - Performance metrics

### Integration
- [x] Quickemu process spawning ✅
- [x] Display protocol detection ✅
- [x] **✅ Quickget integration with edition support** 
- [x] **✅ Tool availability checking and auto-download**
- [ ] File system watching
- [ ] Platform-specific adaptors

### Web UI Implementation - **NEW**
- [ ] Add optional web server feature flag to Cargo.toml
- [ ] Implement HTTP API endpoints for VM operations
- [ ] Create dioxus-app/ directory structure
- [ ] Build Dioxus web frontend components
- [ ] Add shared data models for web/desktop communication
- [ ] Implement web UI VM listing and controls
- [ ] Add real-time status updates via WebSocket/SSE
- [ ] Configure build scripts for both GTK4 and Dioxus builds

## 🐛 Known Issues
- GTK warning: "'titlebar' is not a valid child type" (cosmetic, doesn't affect functionality)
- Real-time metrics not yet displaying actual values
- File watching not yet implemented
- Build warnings for deprecated macros and unused imports (non-critical)

## 💡 Future Ideas
- [ ] VM templates library
- [ ] Snapshot management
- [ ] Network topology view
- [ ] Multi-host support
- [ ] CLI companion tool

## 📝 Notes
- ✅ **Major milestone achieved**: GTK4 composite templates working with signal handling
- ✅ **Application successfully builds and compiles**
- ✅ **Core VM management fully functional**: Complete VM lifecycle management working
- ✅ **Process detection solved**: Robust VM status tracking with fallback methods
- ✅ **Container compatibility**: Works properly in distrobox environments
- ✅ **Quickget integration complete**: Real OS data, edition support, auto-download tools
- ✅ **Production-ready VM creation**: Smart naming, proper OS support, no hardcoded fallbacks
- Focus on MVP features first
- Successfully migrated from Dioxus to GTK4 for better native integration
- All core VM management features implemented (list, start/stop, edit, status tracking)
- Keep binary size minimal
- Prioritize user experience

## 🔥 Immediate Next Steps
1. **✅ COMPLETE: Core VM management** - All basic functionality working
2. **✅ COMPLETE: VM status tracking** - Real-time status updates working
3. **✅ COMPLETE: Process detection** - Robust VM process detection with fallbacks
4. **Implement real-time metrics** - Show actual CPU/RAM usage for running VMs
5. **Add file watching** - Auto-refresh when VM configs change
6. **Persist settings** - Save user preferences and VM directories

## 🎯 Recent Accomplishments
1. **✅ Fixed VM process detection** - Implemented dual detection strategy (sysinfo + ps fallback)
2. **✅ Solved container compatibility** - Application works properly in distrobox environments
3. **✅ Optimized logging** - Clean state-change-only logging with meaningful messages
4. **✅ Button state management** - Start/Stop buttons correctly reflect VM status
5. **✅ Real-time status updates** - Automatic VM status detection every second
6. **✅ Quickget integration overhaul** - Real quickget data instead of hardcoded fallbacks
7. **✅ Tool management system** - Auto-download quickemu/quickget if not available
8. **✅ Edition support** - Proper handling of complex distributions like Fedora
9. **✅ Smart VM naming** - Auto-populate VM names with OS-version-edition pattern
10. **Created comprehensive unit tests** - Added tests for ConfigParser and VMManager
11. **Created integration tests** - Added end-to-end test scenarios
12. **Created justfile** - Added build, test, and development automation commands

## 🌐 Web UI Development Path
1. **Add web server feature** - Optional HTTP API to existing GTK4 app
2. **Create Dioxus structure** - Set up dioxus-app/ directory with minimal web frontend
3. **Implement basic web controls** - VM listing, start/stop, status display
4. **Add real-time updates** - WebSocket or SSE for live VM status
5. **Test and integrate** - Ensure both GTK4 and web UIs work together