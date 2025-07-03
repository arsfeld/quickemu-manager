# SPICE Protocol Reference Manual

## Table of Contents

1. [Protocol Overview](#protocol-overview)
2. [Architecture](#architecture)
3. [Link Protocol](#link-protocol)
4. [Protocol Basics](#protocol-basics)
5. [Channel Types](#channel-types)
6. [Message Format](#message-format)
7. [Authentication](#authentication)
8. [Data Types](#data-types)
9. [Compression Methods](#compression-methods)
10. [Image Formats](#image-formats)
11. [Audio Formats](#audio-formats)
12. [Video Codecs](#video-codecs)
13. [Capabilities](#capabilities)
14. [Migration](#migration)
15. [Error Handling](#error-handling)
16. [Performance Considerations](#performance-considerations)
17. [Extension Guidelines](#extension-guidelines)
18. [Protocol Constants](#protocol-constants)
19. [References](#references)

## Protocol Overview

SPICE (Simple Protocol for Independent Computing Environments) is a remote computing protocol designed to provide efficient access to remote machines. It supports multiple simultaneous connections through different channels, each serving a specific purpose.

### Key Features

- **Multi-channel architecture**: Separate channels for display, input, audio, etc.
- **Efficient compression**: Multiple compression algorithms for different data types
- **Low latency**: Optimized for interactive use
- **Migration support**: Seamless server-to-server migration
- **Extensible**: Capability negotiation for backward compatibility

### Protocol Version

- **Major Version**: 2
- **Minor Version**: 2
- **Latest Updates**: Support for modern video codecs (H.264, H.265, VP8, VP9), improved compression (LZ4), and enhanced streaming capabilities

## Architecture

### Client-Server Model

The SPICE protocol follows a client-server architecture where:

- **SPICE Server**: Runs on the host machine (typically integrated with QEMU/KVM)
- **SPICE Client**: Runs on the user's local machine
- **Channels**: Multiple independent communication channels between client and server

### Connection Flow

1. **Initial Connection**: Client connects to server on main port
2. **Authentication**: Ticket-based authentication using RSA encryption
3. **Capability Negotiation**: Client and server exchange supported features
4. **Channel Setup**: Additional channels are established as needed
5. **Data Exchange**: Bidirectional communication over established channels

### Detailed Startup Sequence

1. **Client → Server**: Send SpiceLinkHeader
   ```c
   SpiceLinkHeader header = {
       .magic = SPICE_MAGIC,
       .major_version = SPICE_VERSION_MAJOR,
       .minor_version = SPICE_VERSION_MINOR,
       .size = sizeof(SpiceLinkMess) + capabilities_size
   };
   ```

2. **Client → Server**: Send SpiceLinkMess
   - Contains channel type, channel ID, and client capabilities

3. **Server → Client**: Send SpiceLinkHeader + SpiceLinkReply
   - Contains error code, RSA public key, and server capabilities

4. **Client → Server**: Send SpiceLinkAuthMechanism
   - Selects authentication method (SPICE or SASL)

5. **Client → Server**: Send authentication data
   - For SPICE auth: RSA-encrypted password/ticket
   - For SASL: SASL authentication exchange

6. **Server → Client**: Send link result
   - On success: Channel is established
   - On failure: Connection is closed with error

## Link Protocol

### Protocol Magic and Version

The SPICE protocol uses a magic number for identification:

```c
#define SPICE_MAGIC         (*(uint32_t*)"REDQ")
#define SPICE_VERSION_MAJOR 2
#define SPICE_VERSION_MINOR 2
```

### Link Header

The initial connection uses this header structure:

```c
typedef struct SpiceLinkHeader {
    uint32_t magic;         // SPICE_MAGIC
    uint32_t major_version; // SPICE_VERSION_MAJOR
    uint32_t minor_version; // SPICE_VERSION_MINOR
    uint32_t size;          // Size of link message
} SpiceLinkHeader;
```

### Link Message

After the header, the client sends a link message:

```c
typedef struct SpiceLinkMess {
    uint32_t connection_id;
    uint8_t  channel_type;
    uint8_t  channel_id;
    uint32_t num_common_caps;
    uint32_t num_channel_caps;
    uint32_t caps_offset;
    // Followed by capabilities array
} SpiceLinkMess;
```

### Link Reply

The server responds with:

```c
typedef struct SpiceLinkReply {
    uint32_t error;         // SpiceLinkErr
    uint8_t  pub_key[SPICE_TICKET_PUBKEY_BYTES];
    uint32_t num_common_caps;
    uint32_t num_channel_caps;
    uint32_t caps_offset;
    // Followed by capabilities array
} SpiceLinkReply;
```

### Authentication Methods

```c
typedef enum {
    SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION = 0,
    SPICE_COMMON_CAP_AUTH_SPICE = 1,
    SPICE_COMMON_CAP_AUTH_SASL = 2,
    SPICE_COMMON_CAP_MINI_HEADER = 3,
} SpiceCommonCap;
```

### Link Authentication

After receiving the public key, the client sends encrypted credentials:

```c
typedef struct SpiceLinkAuthMechanism {
    uint32_t auth_mechanism;  // SPICE_COMMON_CAP_AUTH_*
} SpiceLinkAuthMechanism;

// For SPICE auth:
typedef struct SpiceLinkEncryptedTicket {
    uint8_t encrypted_data[SPICE_TICKET_KEY_PAIR_LENGTH/8];
} SpiceLinkEncryptedTicket;
```

### Security Constants

```c
#define SPICE_TICKET_KEY_PAIR_LENGTH 1024
#define SPICE_TICKET_PUBKEY_BYTES    (SPICE_TICKET_KEY_PAIR_LENGTH/8)
#define SPICE_MAX_PASSWORD_LENGTH    60
```

## Protocol Basics

### Byte Order

All multi-byte values use **little-endian** byte order.

### Basic Types

```c
typedef uint8_t  uint8;
typedef int8_t   int8;
typedef uint16_t uint16;
typedef int16_t  int16;
typedef uint32_t uint32;
typedef int32_t  int32;
typedef uint64_t uint64;
typedef int64_t  int64;
```

### Message Structure

SPICE supports two header formats:

#### Standard Data Header

```c
typedef struct SpiceDataHeader {
    uint64_t serial;     // Message serial number
    uint16_t type;       // Message type
    uint32_t size;       // Size of message body
    uint32_t sub_list;   // Offset to sub-message list (0 if none)
} SpiceDataHeader;
```

#### Mini Header (when SPICE_COMMON_CAP_MINI_HEADER is set)

```c
typedef struct SpiceMiniDataHeader {
    uint16_t type;       // Message type
    uint32_t size;       // Size of message body
} SpiceMiniDataHeader;
```

### Message Constants

```c
#define SPICE_MAX_CHANNELS 64
#define SPICE_MAIN_CHANNEL 1
```

## Channel Types

### 1. Main Channel (ID: 1)

The main channel handles:
- Initial handshake and capability negotiation
- Mouse mode switching
- Agent communication
- Migration coordination

Key Messages:
- `SPICE_MSG_MAIN_INIT`: Server initialization data
- `SPICE_MSG_MAIN_CHANNELS_LIST`: Available channels
- `SPICE_MSG_MAIN_MOUSE_MODE`: Mouse mode configuration
- `SPICE_MSG_MAIN_AGENT_DATA`: Guest agent communication

### 2. Display Channel (ID: 2)

Handles all display-related operations:
- Screen updates and drawing commands
- Surface management
- Video streaming

Key Messages:
- `SPICE_MSG_DISPLAY_MODE`: Display mode changes
- `SPICE_MSG_DISPLAY_DRAW_*`: Various drawing commands
- `SPICE_MSG_DISPLAY_STREAM_*`: Video stream management
- `SPICE_MSG_DISPLAY_SURFACE_*`: Surface operations

### 3. Inputs Channel (ID: 3)

Handles user input:
- Keyboard events
- Mouse movement and clicks

Key Messages:
- `SPICE_MSGC_INPUTS_KEY_DOWN`: Key press
- `SPICE_MSGC_INPUTS_KEY_UP`: Key release
- `SPICE_MSGC_INPUTS_MOUSE_MOTION`: Mouse movement
- `SPICE_MSGC_INPUTS_MOUSE_PRESS`: Mouse button press

### 4. Cursor Channel (ID: 4)

Dedicated channel for cursor updates:
- Cursor shape changes
- Cursor position updates

Key Messages:
- `SPICE_MSG_CURSOR_INIT`: Initial cursor state
- `SPICE_MSG_CURSOR_SET`: Set cursor shape
- `SPICE_MSG_CURSOR_MOVE`: Update cursor position
- `SPICE_MSG_CURSOR_HIDE`: Hide cursor

### 5. Playback Channel (ID: 5)

Audio playback from server to client:
- Audio stream data
- Volume control

Key Messages:
- `SPICE_MSG_PLAYBACK_DATA`: Audio data
- `SPICE_MSG_PLAYBACK_MODE`: Audio format
- `SPICE_MSG_PLAYBACK_START`: Start playback
- `SPICE_MSG_PLAYBACK_VOLUME`: Volume control

### 6. Record Channel (ID: 6)

Audio recording from client to server:
- Microphone input
- Audio capture

Key Messages:
- `SPICE_MSGC_RECORD_DATA`: Audio data
- `SPICE_MSG_RECORD_START`: Start recording
- `SPICE_MSGC_RECORD_MODE`: Audio format

### 7. Tunnel Channel (ID: 7)

Generic tunneling for additional protocols.

### 8. Smartcard Channel (ID: 8)

Smartcard redirection support.

### 9. USB Redirection Channel (ID: 9)

USB device redirection.

### 10. Port Channel (ID: 10)

Generic port forwarding.

### 11. WebDAV Channel (ID: 11)

File sharing via WebDAV protocol.

### Channel Constants

```c
enum {
    SPICE_CHANNEL_MAIN = 1,
    SPICE_CHANNEL_DISPLAY = 2,
    SPICE_CHANNEL_INPUTS = 3,
    SPICE_CHANNEL_CURSOR = 4,
    SPICE_CHANNEL_PLAYBACK = 5,
    SPICE_CHANNEL_RECORD = 6,
    SPICE_CHANNEL_TUNNEL = 7,      // Obsolete
    SPICE_CHANNEL_SMARTCARD = 8,
    SPICE_CHANNEL_USBREDIR = 9,
    SPICE_CHANNEL_PORT = 10,
    SPICE_CHANNEL_WEBDAV = 11
};
```

## Message Format

### Message Header

```c
typedef struct SpiceMsgHeader {
    uint32_t size;       // Total message size (including header)
    uint16_t type;       // Message type
    uint32_t serial;     // Message serial number
    uint32_t sub_list;   // Sub-message list offset
} SpiceMsgHeader;
```

### Message Types

Each channel defines its own message types. Common patterns:

- Messages from server to client: `SPICE_MSG_*`
- Messages from client to server: `SPICE_MSGC_*`

### Sub-messages

Messages can contain sub-messages for efficiency:

```c
typedef struct SpiceSubMessage {
    uint16_t type;
    uint32_t size;
    uint8_t  data[0];   // Variable-length data
} SpiceSubMessage;
```

## Authentication

### Ticket-Based Authentication

1. **Server sends public key**: RSA public key for encryption
2. **Client encrypts ticket**: Uses server's public key
3. **Server validates ticket**: Decrypts and verifies
4. **Connection established**: On successful validation

### Ticket Format

```c
typedef struct SpiceTicket {
    uint8_t password[SPICE_TICKET_KEY_PAIR_LENGTH];
} SpiceTicket;
```

### Security Considerations

- Tickets should be time-limited
- Use strong random passwords
- Consider TLS for additional security

## Data Types

### Point

```c
typedef struct SpicePoint {
    int32_t x;
    int32_t y;
} SpicePoint;
```

### Rectangle

```c
typedef struct SpiceRect {
    int32_t left;
    int32_t top;
    int32_t right;
    int32_t bottom;
} SpiceRect;
```

### Clip

```c
typedef struct SpiceClip {
    uint8_t type;       // SPICE_CLIP_TYPE_*
    union {
        SpiceRect rect;
        uint32_t data;  // Pointer to clip data
    };
} SpiceClip;
```

### Image Descriptor

```c
typedef struct SpiceImageDescriptor {
    uint64_t id;        // Unique image ID
    uint8_t  type;      // Image type
    uint8_t  flags;     // Image flags
    uint32_t width;     // Image width
    uint32_t height;    // Image height
} SpiceImageDescriptor;
```

## Compression Methods

### 1. QUIC

- **Type**: Predictive coding
- **Best for**: Natural images, photos
- **Characteristics**: Good compression ratio, moderate CPU usage

### 2. LZ

- **Type**: LZSS-based compression
- **Best for**: General purpose
- **Characteristics**: Fast, moderate compression

### 3. GLZ

- **Type**: Dictionary-based compression
- **Best for**: Images with repeated patterns
- **Characteristics**: Maintains dictionary across images

### 4. ZLIB

- **Type**: Deflate compression
- **Best for**: Large data blocks
- **Characteristics**: Good compression, higher CPU usage

### 5. LZ4

- **Type**: Fast compression
- **Best for**: Real-time data
- **Characteristics**: Very fast, moderate compression
- **Added**: In newer protocol versions for improved performance

### Compression Type Constants

```c
typedef enum SpiceImageCompression {
    SPICE_IMAGE_COMPRESSION_OFF = 0,
    SPICE_IMAGE_COMPRESSION_AUTO_GLZ = 1,
    SPICE_IMAGE_COMPRESSION_AUTO_LZ = 2,
    SPICE_IMAGE_COMPRESSION_QUIC = 3,
    SPICE_IMAGE_COMPRESSION_GLZ = 4,
    SPICE_IMAGE_COMPRESSION_LZ = 5,
    SPICE_IMAGE_COMPRESSION_LZ4 = 6,
} SpiceImageCompression;
```

## Image Formats

### Bitmap Formats

1. **SPICE_BITMAP_FMT_1BIT_LE**: 1-bit monochrome
2. **SPICE_BITMAP_FMT_1BIT_BE**: 1-bit monochrome (big-endian)
3. **SPICE_BITMAP_FMT_4BIT_LE**: 4-bit indexed
4. **SPICE_BITMAP_FMT_4BIT_BE**: 4-bit indexed (big-endian)
5. **SPICE_BITMAP_FMT_8BIT**: 8-bit indexed
6. **SPICE_BITMAP_FMT_16BIT**: 16-bit RGB (5-6-5)
7. **SPICE_BITMAP_FMT_24BIT**: 24-bit RGB
8. **SPICE_BITMAP_FMT_32BIT**: 32-bit ARGB
9. **SPICE_BITMAP_FMT_RGBA**: 32-bit RGBA

### Surface Formats

1. **SPICE_SURFACE_FMT_1_A**: 1-bit alpha
2. **SPICE_SURFACE_FMT_8_A**: 8-bit alpha
3. **SPICE_SURFACE_FMT_16_555**: 16-bit RGB (5-5-5)
4. **SPICE_SURFACE_FMT_16_565**: 16-bit RGB (5-6-5)
5. **SPICE_SURFACE_FMT_32_xRGB**: 32-bit RGB (no alpha)
6. **SPICE_SURFACE_FMT_32_ARGB**: 32-bit ARGB

## Audio Formats

### Playback/Record Modes

```c
typedef enum SpiceAudioDataMode {
    SPICE_AUDIO_DATA_MODE_RAW = 0,
    SPICE_AUDIO_DATA_MODE_CELT_0_5_1 = 1,  // Deprecated
    SPICE_AUDIO_DATA_MODE_OPUS = 2,
} SpiceAudioDataMode;
```

1. **SPICE_AUDIO_DATA_MODE_RAW**: Uncompressed PCM
2. **SPICE_AUDIO_DATA_MODE_CELT_0_5_1**: CELT compression (deprecated)
3. **SPICE_AUDIO_DATA_MODE_OPUS**: Opus compression (recommended)

### Audio Parameters

```c
typedef struct SpiceAudioFormat {
    uint32_t frequency;  // Sample rate (Hz)
    uint8_t  channels;   // Number of channels
    uint16_t format;     // Sample format
} SpiceAudioFormat;
```

Common sample rates: 44100, 48000 Hz
Common formats: 16-bit signed PCM

## Video Codecs

### Supported Video Codecs

SPICE supports modern video codecs for efficient streaming:

```c
typedef enum SpiceVideoCodecType {
    SPICE_VIDEO_CODEC_TYPE_MJPEG = 0,
    SPICE_VIDEO_CODEC_TYPE_VP8 = 1,
    SPICE_VIDEO_CODEC_TYPE_H264 = 2,
    SPICE_VIDEO_CODEC_TYPE_VP9 = 3,
    SPICE_VIDEO_CODEC_TYPE_H265 = 4,
} SpiceVideoCodecType;
```

### Codec Characteristics

1. **MJPEG (Motion JPEG)**
   - **Type**: Frame-based compression
   - **Best for**: High quality, low latency
   - **Characteristics**: Simple, widely supported, higher bandwidth

2. **VP8**
   - **Type**: Video compression
   - **Best for**: Web streaming
   - **Characteristics**: Good quality/bandwidth ratio, royalty-free

3. **H.264 (AVC)**
   - **Type**: Advanced video compression
   - **Best for**: General purpose streaming
   - **Characteristics**: Excellent compression, hardware acceleration support

4. **VP9**
   - **Type**: Next-gen video compression
   - **Best for**: High-quality streaming
   - **Characteristics**: Better than VP8, royalty-free

5. **H.265 (HEVC)**
   - **Type**: High efficiency video coding
   - **Best for**: 4K/high resolution content
   - **Characteristics**: Best compression ratio, higher CPU requirements

### Stream Configuration

```c
typedef struct SpiceStreamDataHeader {
    uint32_t id;           // Stream ID
    uint32_t multi_media_time;  // Timestamp
    uint32_t data_size;    // Size of stream data
    uint8_t  codec_type;   // SpiceVideoCodecType
} SpiceStreamDataHeader;
```

### Video Stream Messages

- `SPICE_MSG_DISPLAY_STREAM_CREATE`: Create new video stream
- `SPICE_MSG_DISPLAY_STREAM_DATA`: Stream data packet
- `SPICE_MSG_DISPLAY_STREAM_CLIP`: Set stream clipping region
- `SPICE_MSG_DISPLAY_STREAM_DESTROY`: Destroy stream
- `SPICE_MSG_DISPLAY_STREAM_ACTIVATE_REPORT`: Enable stream reports

## Capabilities

### Capability Negotiation

Both client and server advertise their capabilities during connection setup:

```c
typedef struct SpiceMsgMainInit {
    uint32_t session_id;
    uint32_t display_channels_hint;
    uint32_t supported_mouse_modes;
    uint32_t current_mouse_mode;
    uint32_t agent_connected;
    uint32_t agent_tokens;
    uint32_t multi_media_time;
    uint32_t ram_hint;
    uint32_t caps[0];    // Variable-length capability array
} SpiceMsgMainInit;
```

### Capability Definitions

#### Common Capabilities

```c
enum {
    SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION = 0,
    SPICE_COMMON_CAP_AUTH_SPICE = 1,
    SPICE_COMMON_CAP_AUTH_SASL = 2,
    SPICE_COMMON_CAP_MINI_HEADER = 3,
};
```

#### Main Channel Capabilities

```c
enum {
    SPICE_MAIN_CAP_SEMI_SEAMLESS_MIGRATE = 0,
    SPICE_MAIN_CAP_NAME_AND_UUID = 1,
    SPICE_MAIN_CAP_AGENT_CONNECTED_TOKENS = 2,
    SPICE_MAIN_CAP_SEAMLESS_MIGRATE = 3,
};
```

#### Display Channel Capabilities

```c
enum {
    SPICE_DISPLAY_CAP_SIZED_STREAM = 0,
    SPICE_DISPLAY_CAP_MONITORS_CONFIG = 1,
    SPICE_DISPLAY_CAP_COMPOSITE = 2,
    SPICE_DISPLAY_CAP_A8_SURFACE = 3,
    SPICE_DISPLAY_CAP_STREAM_REPORT = 4,
    SPICE_DISPLAY_CAP_LZ4_COMPRESSION = 5,
    SPICE_DISPLAY_CAP_PREF_COMPRESSION = 6,
    SPICE_DISPLAY_CAP_GL_SCANOUT = 7,
    SPICE_DISPLAY_CAP_MULTI_CODEC = 8,
    SPICE_DISPLAY_CAP_CODEC_MJPEG = 9,
    SPICE_DISPLAY_CAP_CODEC_VP8 = 10,
    SPICE_DISPLAY_CAP_CODEC_H264 = 11,
    SPICE_DISPLAY_CAP_PREF_VIDEO_CODEC_TYPE = 12,
    SPICE_DISPLAY_CAP_CODEC_VP9 = 13,
    SPICE_DISPLAY_CAP_CODEC_H265 = 14,
};
```

#### Inputs Channel Capabilities

```c
enum {
    SPICE_INPUTS_CAP_KEY_SCANCODE = 0,
};
```

#### Cursor Channel Capabilities

```c
enum {
    SPICE_CURSOR_CAP_SIZE = 0,
    SPICE_CURSOR_CAP_POSITION = 1,
};
```

#### Playback Channel Capabilities

```c
enum {
    SPICE_PLAYBACK_CAP_CELT_0_5_1 = 0,
    SPICE_PLAYBACK_CAP_VOLUME = 1,
    SPICE_PLAYBACK_CAP_LATENCY = 2,
    SPICE_PLAYBACK_CAP_OPUS = 3,
};
```

#### Record Channel Capabilities

```c
enum {
    SPICE_RECORD_CAP_CELT_0_5_1 = 0,
    SPICE_RECORD_CAP_VOLUME = 1,
    SPICE_RECORD_CAP_OPUS = 2,
};
```

## Migration

### Migration Process

1. **Pre-migration**: Source notifies client of pending migration
2. **Connection to target**: Client connects to target server
3. **State transfer**: Session state transferred to target
4. **Switch-over**: Client switches to target server
5. **Cleanup**: Source connection closed

### Migration Messages

```c
typedef struct SpiceMsgMainMigrationBegin {
    uint16_t port;
    uint16_t sport;
    uint32_t host_offset;
    uint32_t host_size;
    uint32_t cert_subject_offset;
    uint32_t cert_subject_size;
    uint8_t  data[0];    // Host and certificate data
} SpiceMsgMainMigrationBegin;
```

### Seamless vs Semi-seamless

- **Seamless**: No visible interruption to user
- **Semi-seamless**: Brief pause during migration

## Error Handling

### Common Error Codes

- `SPICE_LINK_ERR_OK`: No error
- `SPICE_LINK_ERR_ERROR`: Generic error
- `SPICE_LINK_ERR_INVALID_MAGIC`: Invalid protocol magic
- `SPICE_LINK_ERR_INVALID_DATA`: Invalid data received
- `SPICE_LINK_ERR_VERSION_MISMATCH`: Protocol version mismatch
- `SPICE_LINK_ERR_NEED_SECURED`: Secure connection required
- `SPICE_LINK_ERR_NEED_UNSECURED`: Unsecured connection required
- `SPICE_LINK_ERR_PERMISSION_DENIED`: Authentication failed
- `SPICE_LINK_ERR_BAD_CONNECTION_ID`: Invalid connection ID
- `SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE`: Channel not available

### Error Recovery

1. **Connection errors**: Attempt reconnection with backoff
2. **Protocol errors**: Log and terminate connection
3. **Resource errors**: Graceful degradation where possible

## Performance Considerations

### Bandwidth Optimization

1. **Image caching**: Cache frequently used images
2. **Compression selection**: Choose appropriate compression for data type
3. **Dirty region tracking**: Only send changed screen areas
4. **Video detection**: Use video streaming for motion

### Latency Reduction

1. **Message batching**: Combine small messages
2. **Predictive sending**: Send likely needed data preemptively
3. **Priority queuing**: Prioritize interactive channels
4. **Local rendering**: Render UI elements locally when possible

## Extension Guidelines

### Adding New Channels

1. Define channel ID (must be unique)
2. Define message types for channel
3. Implement capability bits
4. Handle backward compatibility

### Adding New Messages

1. Assign unique message type ID
2. Define message structure
3. Add capability bit if needed
4. Document behavior clearly

### Versioning

- Use capability bits for optional features
- Maintain backward compatibility
- Document version requirements

## Protocol Constants

### Mouse Modes

```c
typedef enum SpiceMouseMode {
    SPICE_MOUSE_MODE_SERVER = 0,
    SPICE_MOUSE_MODE_CLIENT = 1,
} SpiceMouseMode;
```

### Image Types

```c
typedef enum SpiceImageType {
    SPICE_IMAGE_TYPE_BITMAP = 0,
    SPICE_IMAGE_TYPE_QUIC = 1,
    SPICE_IMAGE_TYPE_RESERVED = 2,
    SPICE_IMAGE_TYPE_LZ_PLT = 100,
    SPICE_IMAGE_TYPE_LZ_RGB = 101,
    SPICE_IMAGE_TYPE_GLZ_RGB = 102,
    SPICE_IMAGE_TYPE_FROM_CACHE = 103,
    SPICE_IMAGE_TYPE_SURFACE = 104,
    SPICE_IMAGE_TYPE_JPEG = 105,
    SPICE_IMAGE_TYPE_FROM_CACHE_LOSSLESS = 106,
    SPICE_IMAGE_TYPE_ZLIB_GLZ_RGB = 107,
    SPICE_IMAGE_TYPE_JPEG_ALPHA = 108,
    SPICE_IMAGE_TYPE_LZ4 = 109,
} SpiceImageType;
```

### Notify Severity Levels

```c
typedef enum {
    SPICE_NOTIFY_SEVERITY_INFO = 0,
    SPICE_NOTIFY_SEVERITY_WARN = 1,
    SPICE_NOTIFY_SEVERITY_ERROR = 2,
} SpiceNotifySeverity;
```

### Notify Visibility

```c
typedef enum {
    SPICE_NOTIFY_VISIBILITY_LOW = 0,
    SPICE_NOTIFY_VISIBILITY_MEDIUM = 1,
    SPICE_NOTIFY_VISIBILITY_HIGH = 2,
} SpiceNotifyVisibility;
```

### Common Message Constants

```c
// Base protocol messages
enum {
    SPICE_MSG_MIGRATE = 1,
    SPICE_MSG_MIGRATE_DATA = 2,
    SPICE_MSG_SET_ACK = 3,
    SPICE_MSG_PING = 4,
    SPICE_MSG_WAIT_FOR_CHANNELS = 5,
    SPICE_MSG_DISCONNECTING = 6,
    SPICE_MSG_NOTIFY = 7,
    SPICE_MSG_LIST = 8,
    SPICE_MSG_BASE_LAST = 100,
};

// Client base messages
enum {
    SPICE_MSGC_ACK_SYNC = 1,
    SPICE_MSGC_ACK = 2,
    SPICE_MSGC_PONG = 3,
    SPICE_MSGC_MIGRATE_FLUSH_MARK = 4,
    SPICE_MSGC_MIGRATE_DATA = 5,
    SPICE_MSGC_DISCONNECTING = 6,
    SPICE_MSGC_FIRST_AVAIL = 101,
};
```

### Link Error Codes

```c
typedef enum SpiceLinkErr {
    SPICE_LINK_ERR_OK = 0,
    SPICE_LINK_ERR_ERROR = 1,
    SPICE_LINK_ERR_INVALID_MAGIC = 2,
    SPICE_LINK_ERR_INVALID_DATA = 3,
    SPICE_LINK_ERR_VERSION_MISMATCH = 4,
    SPICE_LINK_ERR_NEED_SECURED = 5,
    SPICE_LINK_ERR_NEED_UNSECURED = 6,
    SPICE_LINK_ERR_PERMISSION_DENIED = 7,
    SPICE_LINK_ERR_BAD_CONNECTION_ID = 8,
    SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE = 9,
} SpiceLinkErr;
```

## References

- [SPICE Protocol Specification](https://www.spice-space.org/spice-protocol.html)
- [SPICE Project Homepage](https://www.spice-space.org/)
- [SPICE Git Repository](https://gitlab.freedesktop.org/spice)
- [SPICE Protocol Headers](https://github.com/flexVDI/spice-protocol)

---

*This reference manual provides a comprehensive overview of the SPICE protocol. For implementation details and code examples, refer to the official SPICE client and server implementations.*