# Critical Issue: SPICE Mini Header Support

## Problem Summary

Our SPICE client implementation has a critical protocol compliance issue:

1. **We advertise `SPICE_COMMON_CAP_MINI_HEADER` support** during capability negotiation
2. **We don't actually implement mini header handling**
3. **We always read 18-byte standard headers**, even when the server may send 6-byte mini headers

This causes protocol desynchronization and connection failures.

## Technical Details

### Standard SPICE Data Header (18 bytes)
```rust
pub struct SpiceDataHeader {
    pub serial: u64,     // 8 bytes
    pub msg_type: u16,   // 2 bytes  
    pub msg_size: u32,   // 4 bytes
    pub sub_list: u32,   // 4 bytes
}  // Total: 18 bytes
```

### Mini Header Format (6 bytes)
```rust
pub struct SpiceMiniDataHeader {
    pub msg_type: u16,   // 2 bytes
    pub msg_size: u32,   // 4 bytes
}  // Total: 6 bytes
```

### Current Implementation Issues

1. In `src/channels/connection.rs:152-153`:
```rust
let common_cap_bits = (1 << SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION) |
                      (1 << SPICE_COMMON_CAP_MINI_HEADER);
```

2. In `src/channels/mod.rs:637-642`:
```rust
pub async fn read_message(&mut self) -> Result<(SpiceDataHeader, Vec<u8>)> {
    const SPICE_DATA_HEADER_SIZE: usize = 18;  // Always reads standard header!
    let header_bytes = self.read_raw(SPICE_DATA_HEADER_SIZE).await?;
    // ...
}
```

3. No storage or checking of negotiated capabilities
4. No conditional header reading based on mini header support

## Impact

- Connection failures with servers that use mini headers
- The E2E test failure (`InvalidData` error) may be related to this issue
- Protocol non-compliance with SPICE specification

## Required Fixes

1. **Add SpiceMiniDataHeader structure** to protocol.rs
2. **Store negotiated capabilities** after link stage
3. **Implement conditional header reading**:
   - Check if mini headers are negotiated
   - Read 6 bytes for mini header or 18 bytes for standard header
4. **Update message writing** to use mini headers when negotiated
5. **Update protocol reference** documentation

## Workaround

As an immediate fix, we could stop advertising mini header support:
```rust
// Only advertise capabilities we actually support
let common_cap_bits = 1 << SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION;
```

This would ensure protocol compliance while we implement proper mini header support.