#!/usr/bin/env node
/**
 * WASM E2E Test Runner for SPICE Client
 * 
 * This script:
 * 1. Starts a local web server to serve WASM files
 * 2. Launches headless Chrome with Playwright
 * 3. Runs WASM tests in the browser environment
 * 4. Reports results
 */

const { chromium } = require('playwright');
const express = require('express');
const path = require('path');
const fs = require('fs').promises;

// Test configuration from environment
const WS_MAIN_URL = process.env.WS_MAIN_URL || 'ws://localhost:8080/main';
const WS_DISPLAY_URL = process.env.WS_DISPLAY_URL || 'ws://localhost:8081/display';
const WS_INPUTS_URL = process.env.WS_INPUTS_URL || 'ws://localhost:8082/inputs';
const WS_CURSOR_URL = process.env.WS_CURSOR_URL || 'ws://localhost:8083/cursor';

const TEST_TIMEOUT = parseInt(process.env.TEST_TIMEOUT || '60000');
const SERVER_PORT = parseInt(process.env.SERVER_PORT || '9000');

// Create test HTML page
const createTestPage = () => `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>SPICE Client WASM Tests</title>
    <style>
        body { font-family: monospace; padding: 20px; }
        #test-canvas { border: 1px solid #ccc; display: block; margin: 20px 0; }
        #log { white-space: pre-wrap; background: #f0f0f0; padding: 10px; }
        .pass { color: green; }
        .fail { color: red; }
        .info { color: blue; }
    </style>
</head>
<body>
    <h1>SPICE Client WASM E2E Tests</h1>
    <canvas id="test-canvas" width="800" height="600"></canvas>
    <div id="log"></div>
    
    <script type="module">
        import init, * as spice from './spice_client.js';
        
        const log = (msg, type = 'info') => {
            const logEl = document.getElementById('log');
            const timestamp = new Date().toISOString();
            logEl.innerHTML += \`<span class="\${type}">[\${timestamp}] \${msg}</span>\\n\`;
            console.log(\`[\${type}] \${msg}\`);
        };
        
        // Test suite
        const tests = {
            async testBasicConnection() {
                log('Testing basic WebSocket connection...', 'info');
                
                const client = await spice.connect_to_server(
                    '${WS_MAIN_URL}',
                    'test-canvas'
                );
                
                // Wait for initialization
                await new Promise(resolve => setTimeout(resolve, 2000));
                
                const state = await client.get_connection_state();
                if (state !== 'connected') {
                    throw new Error(\`Expected connected state, got \${state}\`);
                }
                
                await client.disconnect();
                log('✓ Basic connection test passed', 'pass');
            },
            
            async testMultiChannelConnection() {
                log('Testing multi-channel connection...', 'info');
                
                const config = {
                    main_url: '${WS_MAIN_URL}',
                    display_url: '${WS_DISPLAY_URL}',
                    inputs_url: '${WS_INPUTS_URL}',
                    cursor_url: '${WS_CURSOR_URL}',
                    canvas_id: 'test-canvas'
                };
                
                const client = await spice.connect_multi_channel(config);
                
                // Wait for all channels
                await new Promise(resolve => setTimeout(resolve, 3000));
                
                const channels = await client.get_active_channels();
                if (channels.length < 4) {
                    throw new Error(\`Expected 4 channels, got \${channels.length}\`);
                }
                
                await client.disconnect();
                log('✓ Multi-channel connection test passed', 'pass');
            },
            
            async testInputEvents() {
                log('Testing input events...', 'info');
                
                const client = await spice.connect_to_server(
                    '${WS_MAIN_URL}',
                    'test-canvas'
                );
                
                await new Promise(resolve => setTimeout(resolve, 2000));
                
                // Send keyboard event
                await client.send_key_event(65, true);  // 'A' key down
                await client.send_key_event(65, false); // 'A' key up
                
                // Send mouse events
                await client.send_mouse_move(100, 100);
                await client.send_mouse_click(1, true);  // Left button down
                await client.send_mouse_click(1, false); // Left button up
                
                await client.disconnect();
                log('✓ Input events test passed', 'pass');
            },
            
            async testDisplayUpdates() {
                log('Testing display updates...', 'info');
                
                const client = await spice.connect_to_server(
                    '${WS_MAIN_URL}',
                    'test-canvas'
                );
                
                let frameCount = 0;
                client.on_frame_update(() => {
                    frameCount++;
                });
                
                // Wait for frames
                await new Promise(resolve => setTimeout(resolve, 5000));
                
                if (frameCount === 0) {
                    throw new Error('No frames received');
                }
                
                log(\`Received \${frameCount} frames\`, 'info');
                
                // Capture screenshot
                const imageData = await client.capture_canvas();
                if (!imageData || imageData.length === 0) {
                    throw new Error('Failed to capture canvas');
                }
                
                await client.disconnect();
                log('✓ Display updates test passed', 'pass');
            },
            
            async testReconnection() {
                log('Testing reconnection handling...', 'info');
                
                const client = await spice.connect_to_server(
                    '${WS_MAIN_URL}',
                    'test-canvas'
                );
                
                await new Promise(resolve => setTimeout(resolve, 2000));
                
                // Force disconnect
                await client.disconnect();
                
                // Reconnect
                await client.reconnect();
                
                await new Promise(resolve => setTimeout(resolve, 2000));
                
                const state = await client.get_connection_state();
                if (state !== 'connected') {
                    throw new Error(\`Reconnection failed, state: \${state}\`);
                }
                
                await client.disconnect();
                log('✓ Reconnection test passed', 'pass');
            }
        };
        
        // Run all tests
        async function runTests() {
            await init();
            
            log('Starting WASM E2E tests...', 'info');
            log(\`WebSocket URLs: main=\${WS_MAIN_URL}\`, 'info');
            
            const results = {
                total: 0,
                passed: 0,
                failed: 0,
                errors: []
            };
            
            for (const [name, test] of Object.entries(tests)) {
                results.total++;
                try {
                    await test();
                    results.passed++;
                } catch (error) {
                    results.failed++;
                    results.errors.push({ test: name, error: error.message });
                    log(\`✗ \${name} failed: \${error.message}\`, 'fail');
                }
            }
            
            // Report results
            log('\\n=== Test Results ===', 'info');
            log(\`Total: \${results.total}\`, 'info');
            log(\`Passed: \${results.passed}\`, results.passed > 0 ? 'pass' : 'info');
            log(\`Failed: \${results.failed}\`, results.failed > 0 ? 'fail' : 'info');
            
            // Make results available to Playwright
            window.testResults = results;
        }
        
        // Start tests when page loads
        window.addEventListener('load', runTests);
    </script>
</body>
</html>
`;

// Setup express server
async function setupServer() {
    const app = express();
    
    // Serve WASM files with correct MIME type
    app.use((req, res, next) => {
        if (req.path.endsWith('.wasm')) {
            res.type('application/wasm');
        }
        next();
    });
    
    // Serve test page
    app.get('/', (req, res) => {
        res.type('html').send(createTestPage());
    });
    
    // Serve WASM package files
    const pkgPath = path.join(__dirname, '../../pkg');
    app.use(express.static(pkgPath));
    
    // Start server
    return new Promise((resolve) => {
        const server = app.listen(SERVER_PORT, () => {
            console.log(`Test server running at http://localhost:${SERVER_PORT}`);
            resolve(server);
        });
    });
}

// Run tests with Playwright
async function runTests() {
    let server;
    let browser;
    
    try {
        // Start web server
        server = await setupServer();
        
        // Launch browser
        console.log('Launching Chrome...');
        browser = await chromium.launch({
            headless: true,
            args: ['--no-sandbox', '--disable-setuid-sandbox']
        });
        
        const context = await browser.newContext();
        const page = await context.newPage();
        
        // Enable console logging
        page.on('console', msg => {
            console.log(`[Browser] ${msg.text()}`);
        });
        
        page.on('pageerror', error => {
            console.error(`[Browser Error] ${error.message}`);
        });
        
        // Navigate to test page
        console.log(`Navigating to test page...`);
        await page.goto(`http://localhost:${SERVER_PORT}`, {
            waitUntil: 'networkidle',
            timeout: 30000
        });
        
        // Wait for tests to complete
        console.log('Running tests...');
        const results = await page.waitForFunction(
            () => window.testResults !== undefined,
            { timeout: TEST_TIMEOUT }
        ).then(() => page.evaluate(() => window.testResults));
        
        // Print results
        console.log('\n=== Final Test Results ===');
        console.log(`Total tests: ${results.total}`);
        console.log(`Passed: ${results.passed}`);
        console.log(`Failed: ${results.failed}`);
        
        if (results.failed > 0) {
            console.error('\nFailed tests:');
            results.errors.forEach(({ test, error }) => {
                console.error(`  - ${test}: ${error}`);
            });
            process.exit(1);
        }
        
        console.log('\nAll tests passed! ✓');
        process.exit(0);
        
    } catch (error) {
        console.error('Test runner error:', error);
        process.exit(1);
    } finally {
        if (browser) await browser.close();
        if (server) server.close();
    }
}

// Handle errors
process.on('unhandledRejection', (error) => {
    console.error('Unhandled rejection:', error);
    process.exit(1);
});

// Run tests
runTests();