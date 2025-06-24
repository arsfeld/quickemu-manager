# Quickemu Manager - Development TODO

## 🚀 Current Sprint

### Phase 1: Foundation
- [x] Create product guidelines
- [x] Create technical briefing
- [ ] Initialize Rust project with Dioxus
- [ ] Set up basic project structure
- [ ] Implement quickemu config parser
- [ ] Create VM discovery service
- [ ] Build core UI components

### Phase 2: Core Features
- [ ] VM lifecycle management (start/stop/restart)
- [ ] Process monitoring integration
- [ ] Real-time metrics collection
- [ ] File watching for auto-refresh
- [ ] Settings persistence

### Phase 3: Advanced Features
- [ ] Quickget integration
- [ ] Display protocol launching
- [ ] Resource usage charts
- [ ] VM creation wizard
- [ ] Error handling and recovery

### Phase 4: Polish
- [ ] Platform-specific theming
- [ ] Performance optimization
- [ ] Binary packaging
- [ ] Documentation
- [ ] Release preparation

## 📋 Detailed Tasks

### Project Setup
- [ ] Create Cargo.toml with dependencies
- [ ] Set up Dioxus with desktop feature
- [ ] Configure build scripts
- [ ] Add development documentation

### Core Implementation
- [ ] `config/parser.rs` - Parse quickemu .conf files
- [ ] `services/vm_manager.rs` - VM lifecycle management
- [ ] `services/discovery.rs` - Find and monitor VMs
- [ ] `services/process.rs` - Process monitoring
- [ ] `services/metrics.rs` - Resource usage tracking

### UI Components
- [ ] `components/vm_card.rs` - VM display card
- [ ] `components/vm_list.rs` - VM gallery view
- [ ] `components/vm_detail.rs` - Detailed VM view
- [ ] `components/metrics_chart.rs` - Resource graphs
- [ ] `components/settings.rs` - App settings

### Models
- [ ] `models/vm.rs` - VM data structures
- [ ] `models/config.rs` - App configuration
- [ ] `models/metrics.rs` - Performance metrics

### Integration
- [ ] Quickemu process spawning
- [ ] Display protocol detection
- [ ] File system watching
- [ ] Platform-specific adaptors

## 🐛 Known Issues
- None yet

## 💡 Future Ideas
- [ ] VM templates library
- [ ] Snapshot management
- [ ] Network topology view
- [ ] Multi-host support
- [ ] CLI companion tool

## 📝 Notes
- Focus on MVP features first
- Ensure cross-platform compatibility
- Keep binary size minimal
- Prioritize user experience