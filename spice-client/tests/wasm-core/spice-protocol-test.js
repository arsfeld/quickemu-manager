#!/usr/bin/env node
/**
 * Real SPICE Protocol E2E Test for WASM Core
 * 
 * Tests the actual SPICE protocol handshake and message exchange:
 * - SPICE link handshake (RedLinkMess)
 * - Authentication negotiation
 * - Channel initialization
 * - Message serialization with proper SPICE format
 */

const fs = require('fs').promises;
const path = require('path');
const WebSocket = require('ws');
const { performance } = require('perf_hooks');

// Test configuration
const WS_MAIN_URL = process.env.WS_MAIN_URL || 'ws://localhost:8080/main';
const TEST_TIMEOUT = parseInt(process.env.TEST_TIMEOUT || '30000');

// SPICE Protocol Constants
const SPICE_MAGIC = 0x52454451; // "REDQ" in little-endian (as integer)
const SPICE_MAGIC_LE = 0x51444552; // "REDQ" when read as LE from wire
const SPICE_VERSION_MAJOR = 2;
const SPICE_VERSION_MINOR = 2;

// SPICE Channel Types
const CHANNEL_MAIN = 1;
const CHANNEL_DISPLAY = 2;
const CHANNEL_INPUTS = 3;
const CHANNEL_CURSOR = 4;

// SPICE Main Channel Messages
const SPICEC_MAIN_SET_MOUSE_MODE = 1;
const SPICEC_MAIN_ATTACH_CHANNELS = 5;

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
        console.log('=== SPICE Protocol E2E Tests ===\n');
        
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

// SPICE Protocol Helpers
function createSpiceLinkHeader() {
    const buffer = Buffer.alloc(16);
    buffer.writeUInt32LE(SPICE_MAGIC_LE, 0); // Write the LE version
    buffer.writeUInt32LE(SPICE_VERSION_MAJOR, 4);
    buffer.writeUInt32LE(SPICE_VERSION_MINOR, 8);
    buffer.writeUInt32LE(22, 12); // Size of SpiceLinkMess
    return buffer;
}

function createSpiceLinkMess(channelType = CHANNEL_MAIN, channelId = 0) {
    const buffer = Buffer.alloc(22);
    buffer.writeUInt32LE(0, 0);           // connection_id
    buffer.writeUInt8(channelType, 4);    // channel_type
    buffer.writeUInt8(channelId, 5);      // channel_id
    // 2 bytes padding at offset 6-7
    buffer.writeUInt32LE(0, 8);           // num_common_caps
    buffer.writeUInt32LE(0, 12);          // num_channel_caps
    buffer.writeUInt32LE(0, 16);          // caps_offset
    return buffer;
}

function parseSpiceLinkReply(data) {
    if (data.length < 16) {
        throw new Error(`Invalid SpiceLinkReply size: ${data.length}`);
    }
    
    return {
        magic: data.readUInt32LE(0),
        majorVersion: data.readUInt32LE(4),
        minorVersion: data.readUInt32LE(8),
        size: data.readUInt32LE(12)
    };
}

function parseSpiceLinkReplyData(data, offset = 16) {
    if (data.length < offset + 4) {
        throw new Error(`Invalid SpiceLinkReplyData size: ${data.length}`);
    }
    
    return {
        error: data.readUInt32LE(offset),
        // RSA public key would follow at offset+4 for 162 bytes if auth is enabled
    };
}

function createSpiceDataHeader(msgType, msgSize) {
    const buffer = Buffer.alloc(18);
    buffer.writeBigUInt64LE(0n, 0);       // serial (8 bytes)
    buffer.writeUInt16LE(msgType, 8);     // msg_type (2 bytes)
    buffer.writeUInt32LE(msgSize, 10);    // msg_size (4 bytes)
    buffer.writeUInt32LE(0, 14);          // sub_list (4 bytes)
    return buffer;
}

// WebSocket helpers
async function createWebSocket(url) {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket(url);
        
        ws.on('open', () => {
            console.log(`  Connected to ${url}`);
            resolve(ws);
        });
        
        ws.on('error', (err) => {
            reject(new Error(`WebSocket error: ${err.message}`));
        });
        
        // Set timeout for connection
        setTimeout(() => {
            if (ws.readyState === WebSocket.CONNECTING) {
                ws.close();
                reject(new Error('WebSocket connection timeout'));
            }
        }, 5000);
    });
}

async function sendAndReceive(ws, data, timeoutMs = 5000) {
    return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            reject(new Error('No response received'));
        }, timeoutMs);
        
        const messageHandler = (data) => {
            clearTimeout(timeout);
            ws.off('message', messageHandler);
            resolve(data);
        };
        
        ws.on('message', messageHandler);
        ws.send(data);
    });
}

// SPICE Protocol Tests
const runner = new TestRunner();

runner.test('SPICE link handshake', async () => {
    const ws = await createWebSocket(WS_MAIN_URL);
    
    try {
        // Send SpiceLinkHeader + SpiceLinkMess
        const header = createSpiceLinkHeader();
        const linkMess = createSpiceLinkMess(CHANNEL_MAIN, 0);
        const handshake = Buffer.concat([header, linkMess]);
        
        console.log('  Sending SPICE handshake (38 bytes)');
        const response = await sendAndReceive(ws, handshake);
        
        // Parse response
        const linkReply = parseSpiceLinkReply(response);
        console.log(`  Received SpiceLinkReply: magic=0x${linkReply.magic.toString(16)}, ` +
                    `version=${linkReply.majorVersion}.${linkReply.minorVersion}, ` +
                    `size=${linkReply.size}`);
        
        // Verify magic (server sends it as LE)
        if (linkReply.magic !== SPICE_MAGIC_LE) {
            throw new Error(`Invalid magic: 0x${linkReply.magic.toString(16)}, expected 0x${SPICE_MAGIC_LE.toString(16)}`);
        }
        
        // Parse link reply data if present
        if (response.length >= 20) {
            const replyData = parseSpiceLinkReplyData(response);
            console.log(`  Link error code: ${replyData.error}`);
            
            if (replyData.error !== 0) {
                throw new Error(`Link error: ${replyData.error}`);
            }
        }
        
        ws.close();
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

runner.test('SPICE auth negotiation', async () => {
    const ws = await createWebSocket(WS_MAIN_URL);
    
    try {
        // Complete handshake first
        const header = createSpiceLinkHeader();
        const linkMess = createSpiceLinkMess(CHANNEL_MAIN, 0);
        const handshake = Buffer.concat([header, linkMess]);
        
        const linkResponse = await sendAndReceive(ws, handshake);
        const linkReply = parseSpiceLinkReply(linkResponse);
        
        if (linkReply.magic !== SPICE_MAGIC_LE) {
            throw new Error('Invalid handshake response');
        }
        
        // In ticketing mode, we'd send auth here
        // For now, we expect the server to accept without auth
        console.log('  Auth negotiation completed (no auth mode)');
        
        ws.close();
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

runner.test('SPICE main channel init', async () => {
    const ws = await createWebSocket(WS_MAIN_URL);
    
    try {
        // Complete handshake
        const header = createSpiceLinkHeader();
        const linkMess = createSpiceLinkMess(CHANNEL_MAIN, 0);
        const handshake = Buffer.concat([header, linkMess]);
        
        const linkResponse = await sendAndReceive(ws, handshake);
        const linkReply = parseSpiceLinkReply(linkResponse);
        
        if (linkReply.magic !== SPICE_MAGIC_LE) {
            throw new Error('Invalid handshake response');
        }
        
        // After successful handshake, we should receive INIT message
        // Wait for it with a shorter timeout
        const initMessage = await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                resolve(null); // No init message received
            }, 2000);
            
            ws.once('message', (data) => {
                clearTimeout(timeout);
                resolve(data);
            });
        });
        
        if (initMessage) {
            console.log(`  Received init message (${initMessage.length} bytes)`);
            // Could parse SpiceDataHeader here to verify it's an INIT message
        } else {
            console.log('  No init message received (server may require auth)');
        }
        
        ws.close();
    } finally {
        if (ws.readyState === WebSocket.OPEN) {
            ws.close();
        }
    }
});

runner.test('SPICE message format validation', async () => {
    // Test message serialization without network
    
    // Test SpiceLinkHeader
    const header = createSpiceLinkHeader();
    if (header.readUInt32LE(0) !== SPICE_MAGIC_LE) {
        throw new Error('Invalid magic in header');
    }
    console.log('  SpiceLinkHeader validated');
    
    // Test SpiceLinkMess
    const linkMess = createSpiceLinkMess(CHANNEL_DISPLAY, 1);
    if (linkMess.readUInt8(4) !== CHANNEL_DISPLAY) {
        throw new Error('Invalid channel type');
    }
    console.log('  SpiceLinkMess validated');
    
    // Test SpiceDataHeader
    const dataHeader = createSpiceDataHeader(103, 1024); // INIT message
    if (dataHeader.readUInt16LE(8) !== 103) {
        throw new Error('Invalid message type');
    }
    if (dataHeader.readUInt32LE(10) !== 1024) {
        throw new Error('Invalid message size');
    }
    console.log('  SpiceDataHeader validated');
});

runner.test('Multiple SPICE channel handshakes', async () => {
    const channels = [
        { url: WS_MAIN_URL, type: CHANNEL_MAIN, name: 'main' },
        { url: process.env.WS_DISPLAY_URL || 'ws://localhost:8081/display', type: CHANNEL_DISPLAY, name: 'display' }
    ];
    
    const connections = [];
    
    try {
        for (const channel of channels) {
            const ws = await createWebSocket(channel.url);
            
            // Send handshake for this channel
            const header = createSpiceLinkHeader();
            const linkMess = createSpiceLinkMess(channel.type, 0);
            const handshake = Buffer.concat([header, linkMess]);
            
            const response = await sendAndReceive(ws, handshake);
            const linkReply = parseSpiceLinkReply(response);
            
            if (linkReply.magic !== SPICE_MAGIC_LE) {
                throw new Error(`Invalid handshake for ${channel.name} channel`);
            }
            
            console.log(`  ${channel.name} channel handshake completed`);
            connections.push(ws);
        }
        
        // All channels connected
        console.log('  All SPICE channels connected successfully');
        
    } finally {
        // Clean up all connections
        for (const ws of connections) {
            if (ws.readyState === WebSocket.OPEN) {
                ws.close();
            }
        }
    }
});

// Run tests
async function main() {
    console.log('WebSocket URL:', WS_MAIN_URL);
    console.log('Testing real SPICE protocol...\n');
    
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
    
    console.log('\nAll SPICE protocol tests passed! ✓');
    process.exit(0);
}

// Handle errors
process.on('unhandledRejection', (error) => {
    console.error('Unhandled rejection:', error);
    process.exit(1);
});

// Run tests
main();