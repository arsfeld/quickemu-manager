# SPICE Protocol Flow Comparison: Native Rust vs HTML5

## Connection Establishment

### HTML5 Client (from documentation)
1. Creates WebSocket connection with 'binary' subprotocol
2. Sends SpiceLinkHeader with magic "REDQ" (0x51444552)
3. Sends SpiceLinkMess with capabilities
4. Waits for SpiceLinkReply containing RSA public key
5. Encrypts password and sends as SpiceLinkAuthTicket
6. Receives authentication reply

### Native Rust Client (actual implementation)
1. Creates TCP socket connection
2. Sends SpiceLinkHeader: `[82, 69, 68, 81, 2, 0, 0, 0, 2, 0, 0, 0, 24, 0, 0, 0]`
   - Magic: "REDQ" (0x51444552) ✓
   - Major version: 2 ✓
   - Minor version: 2 ✓
   - Size: 24 bytes ✓
3. Sends SpiceLinkMess: `[0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 1, 0, 0, 0]`
   - connection_id: 0
   - channel_type: 1 (Main)
   - channel_id: 0
   - num_common_caps: 1
   - num_channel_caps: 0
   - caps_offset: 20
   - Common capability: 1 (SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION)
4. Receives error response with code 3 (InvalidData)

## Key Differences Found

### 1. Capability Handling
**HTML5**: 
```javascript
// Sets multiple capability bits
const common_cap_bits = (1 << SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION) |
                        (1 << SPICE_COMMON_CAP_MINI_HEADER);
```

**Native Rust**:
```rust
// Only sets AUTH_SELECTION
let common_cap_bits = 1 << SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION;
```

### 2. Message Structure
The native client is sending the correct header but the test-display-no-ssl server might expect:
- Different capability negotiation
- Authentication even when running with no-ssl
- Different message format

### 3. Server Expectations
The debug server (`test-display-no-ssl`) may have different requirements than a standard SPICE server:
- May not support the protocol version being sent
- May require specific capabilities
- May have hardcoded expectations

## Test Results

When running the E2E test:
1. Connection established successfully
2. Link header sent correctly
3. Link message sent with proper format
4. Server responds with error code 3 (InvalidData)

This suggests the issue is in:
- Capability negotiation mismatch
- Server-specific requirements not met
- Protocol version incompatibility

## Recommendations

1. **Check Server Source**: Look at the test-display-no-ssl source to understand its exact requirements
2. **Capability Alignment**: Ensure capabilities match what the server expects
3. **Protocol Version**: May need to adjust protocol version for compatibility
4. **Authentication**: Even no-ssl servers might expect auth flow

## Communication Flow Accuracy

The documented HTML5 flow appears accurate for standard SPICE servers, but the test server has specific requirements that differ from the standard protocol implementation.