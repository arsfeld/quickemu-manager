# SPICE Core Protocol Guide

This guide combines documentation for the four core SPICE protocol headers that form the foundation of the SPICE communication protocol.

## Overview

The SPICE core protocol consists of four essential headers that work together:

1. **types.h** - Basic type definitions (uint32_t, etc.)
2. **macros.h** - Utility macros and compiler attributes
3. **enums.h** - Protocol constants and enumerations
4. **protocol.h** - Core protocol structures and handshake

## Header Dependencies

```
types.h (foundation)
   ↓
macros.h (utilities)
   ↓
enums.h (constants)
   ↓
protocol.h (structures)
```

## 1. Types Foundation (types.h)

### Purpose
Provides platform-independent fixed-width integer types that ensure protocol compatibility across different systems.

### Key Types
- **Unsigned**: `uint8_t`, `uint16_t`, `uint32_t`, `uint64_t`
- **Signed**: `int8_t`, `int16_t`, `int32_t`, `int64_t`
- **Pointer-sized**: `intptr_t`, `uintptr_t`

### Usage Pattern
```c
// Always use fixed-width types in protocol structures
typedef struct {
    uint32_t magic;      // Exactly 32 bits on all platforms
    uint16_t version;    // Exactly 16 bits
    uint8_t  flags;      // Exactly 8 bits
} ProtocolHeader;
```

## 2. Utility Macros (macros.h)

### Compiler Attributes
```c
// Memory allocation hints
void* SPICE_GNUC_MALLOC allocate_buffer(size_t size) SPICE_GNUC_ALLOC_SIZE(1);

// Format string checking
void SPICE_GNUC_PRINTF(2, 3) spice_log(int level, const char *format, ...);

// Deprecation marking
SPICE_GNUC_DEPRECATED void old_function(void);

// Optimization hints
if (SPICE_LIKELY(common_case)) {
    // Fast path
} else if (SPICE_UNLIKELY(error)) {
    // Error handling
}
```

### Endianness Handling
```c
// Create endian-correct magic numbers
#define SPICE_MAGIC SPICE_MAGIC_CONST("REDQ")

// Byte swapping for network order
uint32_t network_val = SPICE_BYTESWAP32(host_val);
```

### Common Utilities
```c
// Array size calculation
size_t count = SPICE_N_ELEMENTS(my_array);

// Container access from member
Container *cont = SPICE_CONTAINEROF(member_ptr, Container, member_field);

// Min/Max/Alignment
aligned_size = SPICE_ALIGN(size, 8);  // Align to 8 bytes
```

## 3. Protocol Constants (enums.h)

### Channel Types
```c
enum {
    SPICE_CHANNEL_MAIN      = 1,
    SPICE_CHANNEL_DISPLAY   = 2,
    SPICE_CHANNEL_INPUTS    = 3,
    SPICE_CHANNEL_CURSOR    = 4,
    SPICE_CHANNEL_PLAYBACK  = 5,
    SPICE_CHANNEL_RECORD    = 6,
    // ... more channels
};
```

### Error Codes
```c
typedef enum {
    SPICE_LINK_ERR_OK                = 0,
    SPICE_LINK_ERR_ERROR             = 1,
    SPICE_LINK_ERR_INVALID_MAGIC     = 2,
    SPICE_LINK_ERR_INVALID_DATA      = 3,
    SPICE_LINK_ERR_VERSION_MISMATCH  = 4,
    // ... more errors
} SpiceLinkErr;
```

### Graphics Types
```c
// Image formats
enum SpiceImageType {
    SPICE_IMAGE_TYPE_BITMAP  = 0,
    SPICE_IMAGE_TYPE_QUIC    = 1,
    SPICE_IMAGE_TYPE_LZ      = 4,
    SPICE_IMAGE_TYPE_JPEG    = 6,
    // ... more formats
};

// Video codecs
enum SpiceVideoCodecType {
    SPICE_VIDEO_CODEC_TYPE_MJPEG = 1,
    SPICE_VIDEO_CODEC_TYPE_VP8   = 2,
    SPICE_VIDEO_CODEC_TYPE_H264  = 3,
    SPICE_VIDEO_CODEC_TYPE_VP9   = 4,
    SPICE_VIDEO_CODEC_TYPE_H265  = 5,
};
```

### Input Constants
```c
// Mouse buttons
enum SpiceMouseButton {
    SPICE_MOUSE_BUTTON_LEFT   = 1,
    SPICE_MOUSE_BUTTON_MIDDLE = 2,
    SPICE_MOUSE_BUTTON_RIGHT  = 3,
    // ... more buttons
};

// Keyboard modifiers
enum SpiceKeyboardModifierFlags {
    SPICE_KEYBOARD_MODIFIER_FLAGS_SHIFT = (1 << 0),
    SPICE_KEYBOARD_MODIFIER_FLAGS_CTRL  = (1 << 1),
    SPICE_KEYBOARD_MODIFIER_FLAGS_ALT   = (1 << 2),
    // ... more modifiers
};
```

## 4. Protocol Structures (protocol.h)

### Connection Handshake

#### Step 1: Client Link Header
```c
SpiceLinkHeader header = {
    .magic = SPICE_MAGIC,                    // "REDQ"
    .major_version = SPICE_VERSION_MAJOR,    // 2
    .minor_version = SPICE_VERSION_MINOR,    // 2
    .size = sizeof(SpiceLinkMess) + caps_size
};
```

#### Step 2: Client Link Message
```c
SpiceLinkMess link_msg = {
    .connection_id = unique_id,
    .channel_type = SPICE_CHANNEL_MAIN,
    .channel_id = 0,
    .num_common_caps = common_caps_count,
    .num_channel_caps = channel_caps_count,
    .caps_offset = sizeof(SpiceLinkMess)
};
// Followed by capability arrays
```

#### Step 3: Server Reply
```c
SpiceLinkReply reply = {
    .error = SPICE_LINK_ERR_OK,
    .pubkey = { /* RSA public key */ },
    .num_common_caps = server_common_caps,
    .num_channel_caps = server_channel_caps,
    .caps_offset = sizeof(SpiceLinkReply)
};
```

#### Step 4: Authentication
```c
SpiceLinkAuthMechanism auth = {
    .auth_mechanism = SPICE_COMMON_CAP_AUTH_SPICE
};

SpiceLinkEncryptedTicket ticket = {
    .encrypted_data = { /* RSA encrypted password */ }
};
```

### Message Format

#### Standard Header
```c
SpiceDataHeader msg = {
    .serial = sequence_number++,
    .type = SPICE_MSG_DISPLAY_DRAW_COPY,
    .size = data_size,
    .sub_list = 0  // No sub-messages
};
```

#### Mini Header (when negotiated)
```c
SpiceMiniDataHeader mini = {
    .type = SPICE_MSG_DISPLAY_DRAW_COPY,
    .size = data_size
};
```

### Capability Management

#### Capability Arrays
```c
// Capabilities stored as bit arrays
uint32_t caps[SPICE_COMMON_CAPS_BYTES / sizeof(uint32_t)];

// Set capability
caps[SPICE_COMMON_CAP_MINI_HEADER / 32] |= (1 << (SPICE_COMMON_CAP_MINI_HEADER % 32));

// Check capability
if (caps[cap / 32] & (1 << (cap % 32))) {
    // Capability is supported
}
```

#### Common Capabilities
- `SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION` - Auth method negotiation
- `SPICE_COMMON_CAP_AUTH_SPICE` - SPICE authentication
- `SPICE_COMMON_CAP_AUTH_SASL` - SASL authentication  
- `SPICE_COMMON_CAP_MINI_HEADER` - Compact message headers

#### Channel-Specific Capabilities
- **Display**: Codecs (MJPEG, VP8, H264, VP9, H265), GL support
- **Audio**: CELT, Opus, volume control
- **Main**: Semi-seamless migration, agent tokens

## Complete Example: Establishing Connection

```c
#include <spice/types.h>
#include <spice/macros.h>
#include <spice/enums.h>
#include <spice/protocol.h>

// 1. Send link header
SpiceLinkHeader header = {
    .magic = SPICE_MAGIC,
    .major_version = SPICE_VERSION_MAJOR,
    .minor_version = SPICE_VERSION_MINOR,
    .size = sizeof(SpiceLinkMess) + sizeof(uint32_t) * 10
};
send(socket, &header, sizeof(header));

// 2. Send link message with capabilities
SpiceLinkMess link_msg = {
    .connection_id = 0,
    .channel_type = SPICE_CHANNEL_MAIN,
    .channel_id = 0,
    .num_common_caps = 10,
    .num_channel_caps = 0,
    .caps_offset = sizeof(SpiceLinkMess)
};
send(socket, &link_msg, sizeof(link_msg));

// Send capabilities
uint32_t caps[10] = {0};
caps[0] |= (1 << SPICE_COMMON_CAP_MINI_HEADER);
caps[0] |= (1 << SPICE_COMMON_CAP_AUTH_SPICE);
send(socket, caps, sizeof(caps));

// 3. Receive server reply
SpiceLinkReply reply;
recv(socket, &reply, sizeof(reply));

if (reply.error != SPICE_LINK_ERR_OK) {
    handle_error(reply.error);
    return;
}

// 4. Send authentication
SpiceLinkAuthMechanism auth = {
    .auth_mechanism = SPICE_COMMON_CAP_AUTH_SPICE
};
send(socket, &auth, sizeof(auth));

// Encrypt password with server's public key and send
SpiceLinkEncryptedTicket ticket;
encrypt_password(password, reply.pubkey, ticket.encrypted_data);
send(socket, &ticket, sizeof(ticket));

// 5. Connection established - begin message exchange
```

## Key Design Principles

1. **Fixed-Width Types**: All protocol fields use exact-size types
2. **Little-Endian**: All multi-byte values in little-endian format
3. **Packed Structures**: Use `SPICE_ATTR_PACKED` for wire format
4. **Capability-Based**: Features negotiated through capability bits
5. **Version Compatibility**: Major version must match, minor indicates features
6. **Security**: RSA-1024 for authentication (consider upgrading)
7. **Extensibility**: New capabilities added at end for compatibility

## Common Patterns

### Error Checking
```c
if (SPICE_UNLIKELY(link_reply.error != SPICE_LINK_ERR_OK)) {
    switch (link_reply.error) {
        case SPICE_LINK_ERR_VERSION_MISMATCH:
            log_error("Protocol version mismatch");
            break;
        case SPICE_LINK_ERR_NEED_SECURED:
            log_error("Secure connection required");
            break;
        // ... handle other errors
    }
}
```

### Message Handling
```c
// Read header
SpiceDataHeader header;
recv(socket, &header, sizeof(header));

// Allocate buffer for message body
void *data = SPICE_GNUC_MALLOC malloc(header.size);

// Read message data
recv(socket, data, header.size);

// Process based on type
switch (header.type) {
    case SPICE_MSG_MAIN_INIT:
        handle_main_init(data);
        break;
    // ... handle other message types
}
```

### Capability Negotiation
```c
// Build capability array
uint32_t my_caps[SPICE_COMMON_CAPS_BYTES / sizeof(uint32_t)] = {0};

// Set supported capabilities
set_capability(my_caps, SPICE_COMMON_CAP_MINI_HEADER);
set_capability(my_caps, SPICE_COMMON_CAP_AUTH_SPICE);

// After handshake, check negotiated capabilities
if (has_capability(negotiated_caps, SPICE_COMMON_CAP_MINI_HEADER)) {
    use_mini_headers = TRUE;
}
```

## Security Considerations

1. **Authentication**: Always use encrypted tickets
2. **Public Key**: Verify server's public key if possible
3. **Password Length**: Maximum 60 characters (`SPICE_MAX_PASSWORD_LENGTH`)
4. **Channel Security**: Some channels may require TLS
5. **Capability Validation**: Verify capability array bounds

## Performance Tips

1. Use `SPICE_LIKELY`/`SPICE_UNLIKELY` for hot paths
2. Enable mini headers when supported to reduce overhead
3. Batch small messages using sub-message lists
4. Use appropriate image compression for bandwidth
5. Align structures for efficient memory access