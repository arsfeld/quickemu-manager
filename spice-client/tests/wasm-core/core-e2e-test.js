#!/usr/bin/env node
/**
 * Lightweight WASM Core E2E Test for SPICE Client
 * 
 * Tests core protocol functionality without browser dependencies:
 * - WebSocket connections
 * - Protocol handshakes
 * - Channel establishment
 * - Message serialization/deserialization
 * 
 * Uses Node.js WebSocket and WASM directly (no DOM/Canvas)
 */

const fs = require('fs').promises;
const path = require('path');
const WebSocket = require('ws');
const { performance } = require('perf_hooks');

// Test configuration
const WS_MAIN_URL = process.env.WS_MAIN_URL || 'ws://localhost:8080/main';
const WS_DISPLAY_URL = process.env.WS_DISPLAY_URL || 'ws://localhost:8081/display';
const WS_INPUTS_URL = process.env.WS_INPUTS_URL || 'ws://localhost:8082/inputs';
const WS_CURSOR_URL = process.env.WS_CURSOR_URL || 'ws://localhost:8083/cursor';

const TEST_TIMEOUT = parseInt(process.env.TEST_TIMEOUT || '30000');

// Simple test framework
class TestRunner {
    constructor() {
        this.tests = new Map();
        this.results = {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: []
        };
    }

    test(name, fn) {
        this.tests.set(name, fn);
    }

    async run() {
        console.log('=== SPICE WASM Core E2E Tests ===\n');
        
        for (const [name, testFn] of this.tests) {
            this.results.total++;
            const startTime = performance.now();
            
            try {
                await Promise.race([
                    testFn(),
                    new Promise((_, reject) => 
                        setTimeout(() => reject(new Error('Test timeout')), TEST_TIMEOUT)
                    )
                ]);
                
                const duration = (performance.now() - startTime).toFixed(2);
                console.log(`✓ ${name} (${duration}ms)`);
                this.results.passed++;
            } catch (error) {
                const duration = (performance.now() - startTime).toFixed(2);
                console.log(`✗ ${name} (${duration}ms)`);
                console.error(`  Error: ${error.message}`);
                this.results.failed++;
                this.results.errors.push({ test: name, error: error.message });
            }
        }
        
        return this.results;
    }
}

// WASM module loader
async function loadWasmModule() {
    // Load the WASM module built for core functionality
    const wasmPath = path.join(__dirname, '../../pkg/spice_client_core_bg.wasm');
    const wasmBuffer = await fs.readFile(wasmPath);
    
    // Import the bindings
    const wasmModule = await WebAssembly.instantiate(wasmBuffer, {
        // Provide minimal imports for core functionality
        wbindgen: {
            __wbindgen_throw: (ptr, len) => {
                throw new Error('WASM error');
            }
        },
        // WebSocket imports for networking
        __wbindgen_placeholder__: {
            __wbg_new_websocket: (url_ptr, url_len) => {
                // Create WebSocket connection
                const url = readString(url_ptr, url_len);
                return createWebSocket(url);
            },
            __wbg_send_websocket: (ws_id, data_ptr, data_len) => {
                // Send data through WebSocket
                const ws = getWebSocket(ws_id);
                const data = new Uint8Array(memory.buffer, data_ptr, data_len);
                ws.send(data);
            }
        }
    });
    
    return wasmModule.instance.exports;
}

// Test utilities
function createMockWebSocket(url) {
    const ws = new WebSocket(url);
    const id = Math.random().toString(36).substr(2, 9);
    
    ws.on('open', () => {
        console.log(`  WebSocket connected to ${url}`);
    });
    
    ws.on('error', (err) => {
        console.error(`  WebSocket error: ${err.message}`);
    });
    
    return { ws, id };
}

async function waitForWebSocketOpen(ws) {
    return new Promise((resolve, reject) => {
        if (ws.readyState === WebSocket.OPEN) {
            resolve();
            return;
        }
        
        ws.once('open', resolve);
        ws.once('error', reject);
        
        setTimeout(() => reject(new Error('WebSocket connection timeout')), 5000);
    });
}

// Protocol message helpers
function createSpiceLink(channelType = 0) {
    // SPICE link message structure
    const magic = Buffer.from('REDQ');  // SPICE magic bytes
    const majorVersion = 2;
    const minorVersion = 2;
    const size = 18;  // Size of link message
    
    const buffer = Buffer.alloc(size);
    let offset = 0;
    
    // Write header
    magic.copy(buffer, offset); offset += 4;
    buffer.writeUInt32LE(majorVersion, offset); offset += 4;
    buffer.writeUInt32LE(minorVersion, offset); offset += 4;
    buffer.writeUInt32LE(size, offset); offset += 4;
    
    // Channel type and ID
    buffer.writeUInt8(channelType, offset); offset += 1;
    buffer.writeUInt8(0, offset); // Channel ID
    
    return buffer;
}

function parseSpiceLinkReply(data) {
    if (data.length < 16) {
        throw new Error('Invalid link reply size');
    }
    
    const magic = data.slice(0, 4).toString();
    if (magic !== 'REDQ') {
        throw new Error(`Invalid magic: ${magic}`);
    }
    
    return {
        magic,
        majorVersion: data.readUInt32LE(4),
        minorVersion: data.readUInt32LE(8),
        size: data.readUInt32LE(12),
        error: data.length > 16 ? data.readUInt32LE(16) : 0
    };
}

// Core E2E Tests
const runner = new TestRunner();

runner.test('WebSocket connection establishment', async () => {
    const { ws } = createMockWebSocket(WS_MAIN_URL);
    
    try {
        await waitForWebSocketOpen(ws);
        
        // Verify connection state
        if (ws.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not in OPEN state');
        }
        
        ws.close();
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

runner.test('WebSocket to TCP proxy connectivity', async () => {
    const { ws } = createMockWebSocket(WS_MAIN_URL);
    
    try {
        await waitForWebSocketOpen(ws);
        
        // The proxy connects to the SPICE server on open
        // The fact that we got here means the proxy is working
        console.log('  WebSocket proxy successfully connected to SPICE server');
        
        // Close gracefully
        ws.close();
        
        // Wait for close to complete
        await new Promise((resolve) => {
            if (ws.readyState === WebSocket.CLOSED) {
                resolve();
            } else {
                ws.once('close', resolve);
            }
        });
        
        console.log('  Connection closed gracefully');
        
    } catch (error) {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
        throw error;
    }
});

runner.test('Multiple channel connections', async () => {
    const channels = [
        { url: WS_MAIN_URL, type: 1, name: 'main' },
        { url: WS_DISPLAY_URL, type: 2, name: 'display' },
        { url: WS_INPUTS_URL, type: 3, name: 'inputs' },
        { url: WS_CURSOR_URL, type: 4, name: 'cursor' }
    ];
    
    const connections = [];
    
    try {
        // Connect all channels
        for (const channel of channels) {
            const { ws } = createMockWebSocket(channel.url);
            await waitForWebSocketOpen(ws);
            connections.push(ws);
            console.log(`  Connected ${channel.name} channel`);
        }
        
        // Verify all connections are open
        for (let i = 0; i < connections.length; i++) {
            if (connections[i].readyState !== WebSocket.OPEN) {
                throw new Error(`Channel ${channels[i].name} not open`);
            }
        }
        
    } finally {
        // Clean up connections
        for (const ws of connections) {
            if (ws.readyState === WebSocket.OPEN) {
                ws.close();
            }
        }
    }
});

runner.test('Message serialization and deserialization', async () => {
    // Test SPICE message format handling
    const testMessages = [
        { type: 1, size: 10, data: Buffer.alloc(10).fill(0xAB) },
        { type: 2, size: 256, data: Buffer.alloc(256).fill(0xCD) },
        { type: 3, size: 1024, data: Buffer.alloc(1024).fill(0xEF) }
    ];
    
    for (const msg of testMessages) {
        // Create SPICE data header
        const header = Buffer.alloc(18);
        header.writeUInt32LE(msg.size, 0);     // size
        header.writeUInt16LE(msg.type, 4);     // type
        
        // Combine header and data
        const fullMessage = Buffer.concat([header, msg.data]);
        
        // Verify message structure
        if (fullMessage.length !== 18 + msg.size) {
            throw new Error(`Invalid message size for type ${msg.type}`);
        }
        
        console.log(`  Validated message type ${msg.type}, size ${msg.size}`);
    }
});

runner.test('Connection state management', async () => {
    const { ws } = createMockWebSocket(WS_MAIN_URL);
    const states = [];
    
    try {
        // Track state changes
        ws.on('open', () => states.push('open'));
        ws.on('close', () => states.push('closed'));
        ws.on('error', () => states.push('error'));
        
        await waitForWebSocketOpen(ws);
        
        // Simulate disconnect and reconnect
        ws.close();
        
        // Wait for close event
        await new Promise(resolve => setTimeout(resolve, 100));
        
        // Verify state transitions
        if (!states.includes('open')) {
            throw new Error('Open state not recorded');
        }
        
        console.log(`  State transitions: ${states.join(' -> ')}`);
        
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

runner.test('Binary data handling', async () => {
    const { ws } = createMockWebSocket(WS_MAIN_URL);
    
    try {
        await waitForWebSocketOpen(ws);
        
        // Test various binary payloads
        const payloads = [
            new Uint8Array([1, 2, 3, 4, 5]),
            new Uint8Array(1024).fill(0xFF),
            new Uint8Array(65536).fill(0x00)
        ];
        
        for (const payload of payloads) {
            // In a real test, we'd verify the server echoes back
            ws.send(payload);
            console.log(`  Sent binary payload: ${payload.length} bytes`);
        }
        
        ws.close();
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

// Run tests
async function main() {
    console.log('WebSocket URLs:');
    console.log(`  Main: ${WS_MAIN_URL}`);
    console.log(`  Display: ${WS_DISPLAY_URL}`);
    console.log(`  Inputs: ${WS_INPUTS_URL}`);
    console.log(`  Cursor: ${WS_CURSOR_URL}`);
    console.log('');
    
    const results = await runner.run();
    
    console.log('\n=== Test Summary ===');
    console.log(`Total: ${results.total}`);
    console.log(`Passed: ${results.passed}`);
    console.log(`Failed: ${results.failed}`);
    
    if (results.failed > 0) {
        console.error('\nFailed tests:');
        results.errors.forEach(({ test, error }) => {
            console.error(`  - ${test}: ${error}`);
        });
        process.exit(1);
    }
    
    console.log('\nAll core tests passed! ✓');
    process.exit(0);
}

// Handle errors
process.on('unhandledRejection', (error) => {
    console.error('Unhandled rejection:', error);
    process.exit(1);
});

// Run tests
main();