#!/usr/bin/env python3
"""
Hot-reload development server for WASM builds.
Serves files with auto-refresh capability.
"""

import os
import sys
import time
import threading
import subprocess
from http.server import HTTPServer, SimpleHTTPRequestHandler
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

class WASMBuildHandler(FileSystemEventHandler):
    """Handles file system events and triggers WASM rebuilds."""
    
    def __init__(self):
        self.last_build_time = 0
        self.build_lock = threading.Lock()
        self.clients = []
        
    def on_modified(self, event):
        if event.is_directory:
            return
            
        # Only rebuild for Rust source files
        if not event.src_path.endswith(('.rs', '.toml')):
            return
            
        # Debounce: avoid multiple rebuilds in quick succession
        current_time = time.time()
        if current_time - self.last_build_time < 2:
            return
            
        with self.build_lock:
            self.last_build_time = current_time
            print(f"\n🔄 Change detected in {event.src_path}")
            self.rebuild()
    
    def rebuild(self):
        """Rebuild the WASM package."""
        print("🔨 Building WASM...")
        try:
            # Set environment to reduce wasm-bindgen verbosity
            env = os.environ.copy()
            env['RUST_LOG'] = 'warn'  # Only show warnings and errors
            
            result = subprocess.run(
                ["wasm-pack", "build", "--target", "web", "--out-dir", "pkg", "--dev"],
                capture_output=True,
                text=True,
                cwd="/app/spice-client",
                env=env
            )
            
            if result.returncode == 0:
                print("✅ Build successful!")
                self.notify_clients()
            else:
                print("❌ Build failed!")
                print(result.stderr)
        except Exception as e:
            print(f"❌ Build error: {e}")
    
    def notify_clients(self):
        """Send reload signal to all connected clients."""
        # This is handled by the SSE endpoint
        pass

class HotReloadHTTPHandler(SimpleHTTPRequestHandler):
    """HTTP handler with Server-Sent Events for hot reload."""
    
    def do_GET(self):
        if self.path == '/sse':
            # Server-Sent Events endpoint for hot reload
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Cache-Control', 'no-cache')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            
            # Keep connection alive and wait for reload events
            while True:
                time.sleep(1)
                # In a real implementation, this would check for rebuild events
                
        elif self.path == '/' or self.path == '/index.html':
            # Serve the development HTML with hot-reload script
            self.serve_dev_html()
        else:
            # Serve other files normally
            super().do_GET()
    
    def serve_dev_html(self):
        """Serve the development HTML with hot-reload capability."""
        html = '''<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>SPICE Client WASM Dev</title>
    <style>
        body { 
            font-family: Arial, sans-serif; 
            max-width: 800px; 
            margin: 0 auto; 
            padding: 20px;
            background: #f5f5f5;
        }
        h1 { color: #333; }
        #status { 
            padding: 10px; 
            margin: 10px 0; 
            border-radius: 5px;
            font-weight: bold;
        }
        .connected { background-color: #4CAF50; color: white; }
        .disconnected { background-color: #f44336; color: white; }
        .connecting { background-color: #ff9800; color: white; }
        .controls { margin: 20px 0; }
        .input-group {
            margin: 10px 0;
        }
        .input-group label {
            display: inline-block;
            width: 120px;
            font-weight: bold;
        }
        .input-group input {
            padding: 5px;
            width: 300px;
            border: 1px solid #ddd;
            border-radius: 3px;
        }
        button {
            padding: 10px 20px;
            font-size: 16px;
            border: none;
            border-radius: 5px;
            background: #2196F3;
            color: white;
            cursor: pointer;
        }
        button:hover { background: #1976D2; }
        #console {
            background: #333;
            color: #0f0;
            padding: 10px;
            border-radius: 5px;
            font-family: monospace;
            min-height: 200px;
            margin-top: 20px;
        }
        .build-status {
            position: fixed;
            top: 10px;
            right: 10px;
            padding: 10px;
            background: #333;
            color: white;
            border-radius: 5px;
            font-size: 12px;
        }
    </style>
</head>
<body>
    <div class="build-status" id="buildStatus">🟢 Ready</div>
    <h1>SPICE Client WASM Development</h1>
    <div id="status" class="disconnected">Disconnected</div>
    
    <div class="controls">
        <div class="input-group">
            <label for="host">SPICE Host:</label>
            <input type="text" id="host" placeholder="e.g., localhost or qemu-spice">
        </div>
        <div class="input-group">
            <label for="port">SPICE Port:</label>
            <input type="text" id="port" value="5900" placeholder="e.g., 5900">
        </div>
        <div class="input-group">
            <label for="password">Password:</label>
            <input type="password" id="password" placeholder="Optional password">
        </div>
        <button id="connect">Connect to SPICE Server</button>
        <button id="disconnect" style="display:none;">Disconnect</button>
    </div>
    
    <div id="console">Waiting for connection...</div>
    
    <script type="module">
        import init, { SpiceClient } from './spice_client.js';
        
        let client;
        const statusEl = document.getElementById('status');
        const consoleEl = document.getElementById('console');
        const connectBtn = document.getElementById('connect');
        const disconnectBtn = document.getElementById('disconnect');
        
        function log(message) {
            const time = new Date().toLocaleTimeString();
            consoleEl.innerHTML += `[${time}] ${message}<br>`;
            consoleEl.scrollTop = consoleEl.scrollHeight;
        }
        
        async function main() {
            try {
                await init();
                log('WASM module loaded successfully');
                
                // Set default host to current hostname
                document.getElementById('host').value = window.location.hostname;
            } catch (e) {
                log('ERROR: Failed to load WASM module: ' + e.message);
            }
            
            connectBtn.addEventListener('click', async () => {
                const host = document.getElementById('host').value || window.location.hostname;
                const port = document.getElementById('port').value || '5900';
                const password = document.getElementById('password').value || '';
                
                statusEl.className = 'connecting';
                statusEl.textContent = 'Connecting...';
                log(`Initiating connection to SPICE server at ${host}:${port}...`);
                
                try {
                    // For WASM, we need to use WebSocket connection through proxy
                    // The proxy accepts target as path: ws://proxy:5959/host:port
                    const wsUrl = `ws://${window.location.hostname}:5959/${host}:${port}`;
                    log(`Connecting via WebSocket proxy to ${host}:${port}`);
                    
                    client = SpiceClient.new_websocket(wsUrl);
                    
                    // Set password if provided
                    if (password) {
                        client.set_password(password);
                    }
                    
                    // Connect to the server
                    await client.connect();
                    
                    // Start the event loop
                    await client.start_event_loop();
                    
                    statusEl.className = 'connected';
                    statusEl.textContent = 'Connected';
                    connectBtn.style.display = 'none';
                    disconnectBtn.style.display = 'inline-block';
                    log(`Successfully connected to SPICE server at ${host}:${port}`);
                } catch (e) {
                    console.error('Connection failed:', e);
                    statusEl.className = 'disconnected';
                    statusEl.textContent = 'Connection failed';
                    log('ERROR: ' + e.message);
                }
            });
            
            disconnectBtn.addEventListener('click', () => {
                if (client) {
                    client.disconnect();
                    client = null;
                }
                statusEl.className = 'disconnected';
                statusEl.textContent = 'Disconnected';
                connectBtn.style.display = 'inline-block';
                disconnectBtn.style.display = 'none';
                log('Disconnected from SPICE server');
            });
        }
        
        main().catch(console.error);
        
        // Hot reload support
        function setupHotReload() {
            let reloadTimer;
            
            function checkForChanges() {
                fetch('/spice_client.js', { method: 'HEAD' })
                    .then(response => {
                        const lastModified = response.headers.get('Last-Modified');
                        const stored = sessionStorage.getItem('lastModified');
                        
                        if (stored && lastModified !== stored) {
                            document.getElementById('buildStatus').textContent = '🔄 Reloading...';
                            setTimeout(() => location.reload(), 500);
                        }
                        
                        sessionStorage.setItem('lastModified', lastModified);
                    })
                    .catch(() => {
                        // Server might be rebuilding
                        document.getElementById('buildStatus').textContent = '🔨 Building...';
                    });
            }
            
            // Check every 2 seconds
            setInterval(checkForChanges, 2000);
            
            // Initial check
            checkForChanges();
        }
        
        setupHotReload();
    </script>
</body>
</html>'''
        
        self.send_response(200)
        self.send_header('Content-Type', 'text/html')
        self.send_header('Content-Length', str(len(html)))
        self.end_headers()
        self.wfile.write(html.encode())

def main():
    """Main entry point."""
    os.chdir('/app/spice-client')
    
    # Initial build
    print("🚀 Starting WASM hot-reload development server...")
    print("🔨 Running initial build...")
    
    # Set environment to reduce wasm-bindgen verbosity
    env = os.environ.copy()
    env['RUST_LOG'] = 'warn'  # Only show warnings and errors
    
    subprocess.run(
        ["wasm-pack", "build", "--target", "web", "--out-dir", "pkg", "--dev"],
        check=True,
        env=env
    )
    
    print("✅ Initial build complete!")
    
    # Set up file watcher
    event_handler = WASMBuildHandler()
    observer = Observer()
    observer.schedule(event_handler, path='/app/spice-client/src', recursive=True)
    observer.start()
    
    # Start HTTP server
    os.chdir('pkg')
    server_address = ('', 8080)
    httpd = HTTPServer(server_address, HotReloadHTTPHandler)
    
    print(f"\n🌐 Development server running at http://localhost:8080")
    print("👀 Watching for changes in /app/spice-client/src...")
    print("\nPress Ctrl+C to stop.\n")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n👋 Shutting down...")
        observer.stop()
        observer.join()
        httpd.shutdown()

if __name__ == '__main__':
    main()