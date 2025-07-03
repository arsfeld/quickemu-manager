# SPICE Client Implementation TODO

## ✅ Completed Protocol Fixes (2025-07-03)

### 1. ~~Authentication Mechanism Selection~~ ✅ COMPLETED
**Fixed**: Client now sends `SpiceLinkAuthMechanism` after receiving server's public key.
**Changes**:
- Added `SpiceLinkAuthMechanism` structure in protocol.rs
- Updated `send_auth()` in connection.rs to send auth mechanism before password
- Now follows correct protocol sequence

### 2. ~~Mini Header Support~~ ✅ COMPLETED
**Fixed**: Removed `SPICE_COMMON_CAP_MINI_HEADER` capability advertisement.
**Changes**:
- Commented out mini header capability in connection.rs
- Added `SpiceMiniDataHeader` structure for future implementation
- Prevents protocol desynchronization with servers

### 3. ~~Incomplete Link Reply Parsing~~ ✅ COMPLETED
**Fixed**: Now properly parses `SpiceLinkReplyData` structure.
**Changes**:
- Updated `wait_for_link_reply()` to parse full structure
- Extracts and stores server capabilities
- Manual byte parsing replaced with proper structure handling

### 4. ~~No Capability Storage~~ ✅ COMPLETED
**Fixed**: Server capabilities are now stored and available for use.
**Changes**:
- Added `server_common_caps` and `server_channel_caps` fields to ChannelConnection
- Capabilities parsed and stored during handshake
- Available for protocol feature detection

### 5. ~~Missing Final Link Result~~ ✅ COMPLETED
**Fixed**: Client now reads and validates link result after authentication.
**Changes**:
- Added `read_link_result()` function in connection.rs
- Reads 4-byte result code after sending password
- Proper error handling for authentication failures

## Additional Completed Work

### 6. ~~Bincode to Binrw Migration~~ ✅ COMPLETED
**Fixed**: Replaced bincode with binrw throughout the codebase.
**Changes**:
- Removed bincode from Cargo.toml
- Updated all serialization/deserialization to use binrw
- Updated error handling to use binrw error types
- More appropriate for protocol implementations

## Next Steps for Full Display Capabilities (HIGH PRIORITY)

### 1. Fix SPICE_MSGC_DISPLAY_INIT Message Format
**Issue**: Server fails to parse message type 101 (SPICE_MSGC_DISPLAY_INIT)
**Required**:
- Research exact format expected by SPICE servers
- Implement proper `SpiceMsgcDisplayInit` structure with:
  - `cache_id` (proper type - likely u8)
  - `cache_size` (i64)
  - `glz_dict_id` (u8)
- Update display.rs to send properly formatted message
- Test against both debug and QEMU servers

### 2. Handle Server Display Messages
**Required Messages to Implement**:
- `SPICE_MSG_DISPLAY_MODE` (101) - Display mode configuration
- `SPICE_MSG_DISPLAY_MARK` (102) - Display mark
- `SPICE_MSG_DISPLAY_RESET` (103) - Display reset
- `SPICE_MSG_DISPLAY_SURFACE_CREATE` (318) - Surface creation
- `SPICE_MSG_DISPLAY_SURFACE_DESTROY` (319) - Surface destruction
- `SPICE_MSG_DISPLAY_MONITORS_CONFIG` (320) - Monitor configuration

### 3. Implement Surface Management
**Required**:
- Handle primary surface creation
- Track surface IDs and properties
- Implement surface format conversions
- Handle surface updates and invalidations

### 4. Implement Drawing Commands
**Priority Drawing Commands**:
- `SPICE_MSG_DISPLAY_DRAW_FILL` (302)
- `SPICE_MSG_DISPLAY_DRAW_COPY` (304)
- `SPICE_MSG_DISPLAY_DRAW_BLEND` (305)
- `SPICE_MSG_DISPLAY_DRAW_TRANSPARENT` (312)
- `SPICE_MSG_DISPLAY_DRAW_ALPHA_BLEND` (317)

### 5. Implement Image Decoding
**Required Decoders**:
- BITMAP (uncompressed)
- QUIC compression
- LZ/GLZ compression
- JPEG support
- LZ4 compression (if capability advertised)

### 6. Implement Cursor Channel Properly
**Required**:
- Handle `SPICE_MSG_CURSOR_INIT` (101)
- Handle `SPICE_MSG_CURSOR_SET` (103)
- Handle `SPICE_MSG_CURSOR_MOVE` (104)
- Handle `SPICE_MSG_CURSOR_HIDE` (105)

### 7. Implement Proper Capability Handling
**Required**:
- Store and use negotiated capabilities
- Conditionally enable features based on capabilities
- Handle display channel specific capabilities

### 8. Fix Channel Connection Sequence
**Issue**: Cursor channel connection fails after display channel timeout
**Required**:
- Ensure proper channel initialization order
- Handle channel dependencies correctly
- Implement proper error recovery

## Lower Priority Tasks

### SASL Authentication Support (LOW PRIORITY)
- No SASL authentication structures
- No SASL mechanism handling
- Only SPICE (RSA) authentication implemented

### Mini Header Implementation (MEDIUM PRIORITY)
- Structure is defined but not implemented
- Would require conditional header reading based on capabilities
- Needed for bandwidth optimization

### Video Streaming Support (LOW PRIORITY)
- Handle `SPICE_MSG_DISPLAY_STREAM_CREATE`
- Handle `SPICE_MSG_DISPLAY_STREAM_DATA`
- Implement video codec support (MJPEG, H264, VP8/9)

### Test File Updates
- Update test files to use binrw instead of bincode
- Files affected: client_tests.rs, display_tests.rs, tests/mocks/mod.rs

## Protocol Sequence (Now Implemented Correctly)

### Current Implementation:
1. ✅ Client → Server: `SpiceLinkHeader`
2. ✅ Client → Server: `SpiceLinkMess` + capabilities
3. ✅ Server → Client: `SpiceLinkHeader` + `SpiceLinkReplyData` + capabilities
4. ✅ Client → Server: `SpiceLinkAuthMechanism` (select SPICE auth)
5. ✅ Client → Server: Encrypted password
6. ✅ Server → Client: Link result (4 bytes)

All critical protocol compliance issues have been resolved!

## Testing Requirements:

- Test against standard SPICE servers (QEMU/KVM)
- Verify protocol compliance with packet captures
- Test capability negotiation with different server configurations
- Ensure backward compatibility is maintained

## Summary of Changes:

The SPICE client now correctly implements the protocol handshake sequence:
- Proper authentication mechanism selection
- Complete link reply parsing with capability storage
- Link result validation after authentication
- Removed unsupported mini header capability advertisement
- Migrated from bincode to binrw for better protocol control

These fixes should resolve connection issues with standard SPICE servers.