# Debugging the Dioxus Web UI

This guide explains how to debug the Dioxus web application with full console output visibility and browser developer tools integration.

## Quick Start

1. **Start the debug server:**
   ```bash
   cd dioxus-app
   ./debug_server.sh
   ```
   This starts the Dioxus web server on http://localhost:8081 with full debug logging.

2. **Open browser with developer tools:**
   ```bash
   # In another terminal
   cd dioxus-app
   cargo run --bin browser_debug
   ```
   This automatically detects the server and provides instructions for opening the browser.

## Current Working Setup

### ✅ Successfully Running
- **Server**: http://localhost:8081 (Dioxus web platform)
- **Debug Tool**: Auto-detects server status and provides browser instructions
- **Hot Reload**: Active with `dx serve --platform web`
- **Full Logging**: Console output visible with environment variables set

### Debug Tools

#### `debug_server.sh` (Updated & Working)
Modern script using `dx serve --platform web`:
- Uses Dioxus CLI for proper web platform support
- Output visible in terminal with hot reload
- Logs saved to `debug_output.log`
- Environment variables:
  - `RUST_LOG=debug,quickemu_manager_ui=trace,quickemu_core=debug,dioxus=debug,axum=debug`
  - `RUST_BACKTRACE=1`
  - `DIOXUS_LOG=trace`

#### `browser_debug` binary (Updated)
Smart detection tool that:
- Checks both server (8081) and client (3001) endpoints
- Auto-detects fullstack vs web-only mode
- Opens browser with developer tools on correct URL
- Works on Linux, macOS, and Windows
- Provides debugging tips and log file location

## Debugging Workflow

### 1. Server-Side Debugging

**View server logs in real-time:**
```bash
# All output appears in the terminal where you ran ./debug_server.sh
# You can also tail the log file:
tail -f debug_output.log

# Or view recent output
tail -20 debug_output.log
```

**Common server-side issues to look for:**
- Dioxus platform detection errors
- Port conflicts (solution: use different port in script)
- WASM compilation issues
- Asset loading problems
- CSS/JavaScript bundle errors

### 2. Client-Side Debugging

**Access the web UI:**
- Direct browser access: http://localhost:8081
- Use proper Accept headers for HTML content:
  ```bash
  curl -H "Accept: text/html,application/xhtml+xml,application/xml" http://localhost:8081
  ```

**Browser Developer Tools:**
- **Console Tab**: JavaScript errors, WASM logs, Dioxus framework messages
- **Network Tab**: Asset loading (JS/WASM bundles), hot reload WebSockets
- **Elements Tab**: Inspect generated HTML, component structure
- **Sources Tab**: JavaScript/WASM debugging (limited WASM support)
- **Performance Tab**: Rendering performance, component re-renders

**Enable verbose client logging:**
```rust
#[cfg(target_arch = "wasm32")]
web_sys::console::log_1(&format!("Debug: {}", message).into());

// Or use Dioxus logging
log::debug!("Component rendered: {}", component_name);
```

### 3. Hot Reload Debugging

**Monitor hot reload:**
- Watch terminal output for file change detection
- Check browser console for hot reload messages
- Look for "Your app is being rebuilt" toast notifications
- Verify WebSocket connection in Network tab

**Hot reload issues:**
- Non-hot-reloadable changes require full rebuild
- WASM compilation errors prevent hot reload
- Asset changes may need manual refresh

## Environment Variables for Debugging

```bash
# Current working setup (used in debug_server.sh)
export RUST_LOG=debug,quickemu_manager_ui=trace,quickemu_core=debug,dioxus=debug,axum=debug
export RUST_BACKTRACE=1
export DIOXUS_LOG=trace

# Maximum verbosity for troubleshooting
export RUST_LOG=trace
export RUST_BACKTRACE=full

# Specific module debugging
export RUST_LOG=quickemu_manager_ui::components=trace
export RUST_LOG=dioxus_web=debug
```

## Common Issues and Solutions

### 1. **Platform Detection Errors**
```
ERROR: No platform was specified and could not be auto-detected
```
**Solution**: Use `dx serve --platform web` (already implemented in debug_server.sh)

### 2. **Port Conflicts**
```
ERROR: Failed to bind server to: 0.0.0.0:8081, is there another devserver running?
```
**Solution**: 
- Kill existing processes: `pkill -f "dx serve"`
- Or change port in debug_server.sh: `dx serve --platform web --port 8082`

### 3. **WASM Loading Issues**
```
Failed to fetch WASM module
```
**Solution**:
- Check browser console for specific errors
- Verify WASM files are being served correctly
- Check Network tab for 404s on .wasm files

### 4. **CSS/Styling Not Loading**
**Solution**:
- Verify assets/style.css exists and is included
- Check for CSS compilation errors in terminal
- Inspect Elements tab to see if styles are applied

### 5. **Hot Reload Not Working**
**Solution**:
- Check WebSocket connection in Network tab
- Look for file watcher errors in terminal
- Restart debug server if needed

## Current Working Commands

```bash
# Start debug server (working setup)
cd dioxus-app && ./debug_server.sh

# Open browser debug tool
cd dioxus-app && cargo run --bin browser_debug

# Manual browser access
open http://localhost:8081
# or
curl -H "Accept: text/html" http://localhost:8081

# Check server status
curl -I http://localhost:8081

# View logs
tail -f debug_output.log

# Kill debug server
pkill -f "dx serve"
```

## Development Tips

1. **Use structured logging:**
   ```rust
   tracing::debug!(component = "VMCard", vm_id = ?id, "Rendering VM card");
   ```

2. **Debug component lifecycle:**
   ```rust
   use_effect(|| {
       log::debug!("Component mounted");
       move || log::debug!("Component unmounted")
   });
   ```

3. **Monitor state changes:**
   ```rust
   let state = use_signal(|| initial_value);
   use_effect(move || {
       log::debug!("State changed: {:?}", state.read());
   });
   ```

4. **Browser debugging helpers:**
   - Use browser console: `window.__DIOXUS__` for framework info
   - Network throttling for testing slow connections
   - Disable cache during development

## Building for Production

When ready for production:
```bash
# Build optimized version
dx build --release

# Output will be in dist/ directory
ls -la dist/

# Serve static files (for testing)
python -m http.server 8000 -d dist/
```

## Successfully Resolved Issues

✅ **Platform Detection**: Fixed by using `--platform web`  
✅ **Port Conflicts**: Using port 8081 instead of default 8080  
✅ **Browser Integration**: Smart debug tool with auto-detection  
✅ **Hot Reload**: Working with proper Dioxus CLI setup  
✅ **Asset Loading**: CSS, JS, and WASM bundles loading correctly  

The debug setup is now fully functional and ready for AI-assisted development!