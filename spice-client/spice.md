# SPICE Protocol Definition v2.2

## Overview
SPICE (Simple Protocol for Independent Computing Environments) provides remote access to computing devices across networks. It uses simple messaging without RPC dependencies and supports multiple communication channels for different device types.

## Handshake Flow

### Phase 1: Link Stage (Per-Channel Connection)

**Step 1 - Client → Server: SpiceLinkMess**
```
UINT32 magic            = 0x52454451 ("REDQ")
UINT32 major_version    = 2
UINT32 minor_version    = 2
UINT32 size            (bytes following this field)
UINT32 connection_id   (0 for new session/main channel, 
                        session_id for other channels)
UINT8  channel_type    (MAIN=1, DISPLAY=2, etc.)
UINT8  channel_id      
UINT32 num_common_caps 
UINT32 num_channel_caps
UINT32 caps_offset     
[capabilities vector]
```

**Step 2 - Server → Client: SpiceLinkReply**
```
UINT32 magic            = 0x52454451 ("REDQ")
UINT32 major_version    (server version)
UINT32 minor_version    
UINT32 size            
UINT32 error           (0=OK, else error code)
UINT8[162] pub_key     (1024-bit RSA public key, X.509 format)
UINT32 num_common_caps
UINT32 num_channel_caps
UINT32 caps_offset
[capabilities vector]
```

**Step 3 - Client → Server: Encrypted Password**
- Always sent, even if password is empty
- Encrypted using RSA-OAEP (PKCS#1 v2.0) with SHA-1, MGF1
- Uses public key from Step 2

**Step 4 - Server → Client: Link Result**
```
UINT32 error_code      (SPICE_LINK_ERR_OK=0 for success)
```

### Phase 2: Channel-Specific Initialization

**Main Channel (must be first):**
Server → Client: SPICE_MSG_MAIN_INIT (type=103)
```
SpiceDataHeader {
    UINT64 serial = 1
    UINT16 type = 103
    UINT32 size
    UINT32 sub_list = 0
}
SpiceMsgMainInit {
    UINT32 session_id          (new session ID)
    UINT32 display_channels_hint
    UINT32 supported_mouse_modes
    UINT32 current_mouse_mode
    UINT32 agent_connected
    UINT32 agent_tokens
    UINT32 multi_media_time
    UINT32 ram_hint
}
```

**Other Channels:**
- Display: SPICE_MSG_DISPLAY_MODE → Set display parameters
- Inputs: SPICE_MSG_INPUTS_INIT → Set keyboard LED state
- Cursor: SPICE_MSG_CURSOR_INIT → Set cursor state and clear cache
- Playback: SPICE_MSG_PLAYBACK_MODE → Set audio format

### Important Handshake Rules
1. Main channel MUST be established first
2. Only one main channel allowed per session
3. Password step is mandatory (even if empty)
4. All messages after handshake use SpiceDataHeader
5. Serial numbers start at 1 and increment per message

## Core Components

### Protocol Structure
- **Magic**: `0x52, 0x45, 0x44, 0x51` ("REDQ")
- **Version**: Major=2, Minor=2
- **Endianness**: Little-endian (unless stated otherwise)
- **Data Packing**: All structures are packed

### Base Data Types
```
UINT8       8-bit unsigned
INT16      16-bit signed  
UINT16     16-bit unsigned
UINT32     32-bit unsigned
INT32      32-bit signed
UINT64     64-bit unsigned
SPICE_ADDRESS   64-bit offset from message body
SPICE_FIXED28_4 32-bit fixed point (28.4)
```

### Geometric Types
```
POINT      {INT32 x, y}
POINT16    {INT16 x, y}
RECT       {INT32 top, left, bottom, right}
POINTFIX   {SPICE_FIXED28_4 x, y}
```

## Ticketing System

### Overview
Ticketing provides time-limited authentication for SPICE connections:
- Server generates RSA keypair per connection
- Ticket consists of password + time validity
- Client must connect within time window
- Empty passwords still validated against time

### Configuration
- Ticket can be empty (passwordless) but still time-checked
- Server controls acceptance of empty passwords
- Time validity prevents replay attacks

## Connection Process (Detailed)

### 1. Link Stage (Removed - now in Handshake Flow section)

### 2. Message Stage
**SpiceDataHeader** (all messages)
```
UINT64 serial       (starts at 1)
UINT16 type        
UINT32 size        
UINT32 sub_list    (offset to sub-messages)
```

## Channel Types
```
SPICE_CHANNEL_MAIN        = 1
SPICE_CHANNEL_DISPLAY     = 2
SPICE_CHANNEL_INPUTS      = 3
SPICE_CHANNEL_CURSOR      = 4
SPICE_CHANNEL_PLAYBACK    = 5
SPICE_CHANNEL_RECORD      = 6
SPICE_CHANNEL_SMARTCARD   = 8
SPICE_CHANNEL_USBREDIR    = 9
SPICE_CHANNEL_PORT        = 10
SPICE_CHANNEL_WEBDAV      = 11
```

## Migration Process

### Overview
SPICE supports live migration of sessions between servers with minimal disruption.

### Migration Flow
1. **Main Channel Initiates**: Server sends SPICE_MSG_MAIN_MIGRATE_BEGIN
2. **Client Prepares**: Connects to destination server
3. **Client Confirms**: Sends SPICE_MSGC_MAIN_MIGRATE_CONNECTED
4. **Per-Channel Migration**:
   - Server sends SPICE_MSG_MIGRATE with flags
   - If NEED_FLUSH: Client sends SPICE_MSGC_MIGRATE_FLUSH_MARK
   - If NEED_DATA_TRANSFER: Server sends SPICE_MSG_MIGRATE_DATA
   - Client switches to destination
   - Client sends SPICE_MSGC_MIGRATE_DATA to destination
5. **Completion**: All channels migrated, source connection closed

### Migration Messages
```
SpiceMsgMainMigrationBegin {
    UINT16 port
    UINT16 sport         (secure port)
    UINT8[] host_name
}

SpiceMsgMigrate {
    UINT32 flags         (NEED_FLUSH=1, NEED_DATA_TRANSFER=2)
}
```

## Synchronization Mechanisms

### Message Acknowledgment
Flow control to prevent channel flooding:
```
SpiceMsgSetAck {
    UINT32 generation    (sequence number)
    UINT32 window       (messages before ACK needed)
}
```
- Client sends SPICE_MSGC_ACK every 'window' messages
- Window=0 disables acknowledgments

### Channel Synchronization
Wait for specific messages across channels:
```
SpiceMsgWaitForChannels {
    UINT8 wait_count
    SpiceWaitForChannel[] {
        UINT8 type
        UINT8 id
        UINT64 serial
    }
}
```

### Multimedia Time
Synchronization for audio/video:
- Carried in playback data messages
- Updated via SPICE_MSG_MAIN_MULTI_MEDIA_TIME
- Enables A/V sync across channels

## Common Messages

### Server → Client
- `SPICE_MSG_MIGRATE`: Channel migration
- `SPICE_MSG_SET_ACK`: Request acknowledgments
- `SPICE_MSG_PING`: Round-trip time measurement
- `SPICE_MSG_WAIT_FOR_CHANNELS`: Synchronization
- `SPICE_MSG_DISCONNECTING`: Orderly disconnect
- `SPICE_MSG_NOTIFY`: Notifications

### Client → Server
- `SPICE_MSGC_ACK_SYNC`: Acknowledge sync
- `SPICE_MSGC_ACK`: Message acknowledgment
- `SPICE_MSGC_PONG`: Ping response
- `SPICE_MSGC_MIGRATE_FLUSH_MARK`: Migration flush
- `SPICE_MSGC_MIGRATE_DATA`: Migration data
- `SPICE_MSGC_DISCONNECTING`: Client disconnect

## Main Channel

### Initialization
**SPICE_MSG_MAIN_INIT** (Must be first message)
```
UINT32 session_id              (server-generated)
UINT32 display_channels_hint   (0=invalid)
UINT32 supported_mouse_modes   (bitmask)
UINT32 current_mouse_mode     
UINT32 agent_connected        (0/1)
UINT32 agent_tokens           (flow control)
UINT32 multi_media_time      
UINT32 ram_hint               (for compression)
```

### Client Information
**SPICE_MSGC_MAIN_CLIENT_INFO** (type=101)
Sent after init to provide client details

### Mouse Modes
- `SPICE_MOUSE_MODE_SERVER = 1`: Server controls cursor
- `SPICE_MOUSE_MODE_CLIENT = 2`: Client controls cursor

### Migration Control
- `SPICE_MSG_MAIN_MIGRATE_BEGIN`: Start migration
- `SPICE_MSG_MAIN_MIGRATE_CANCEL`: Cancel migration
- `SPICE_MSGC_MAIN_MIGRATE_CONNECTED`: Migration ready
- `SPICE_MSGC_MAIN_MIGRATE_CONNECT_ERROR`: Migration failed

### Agent Communication
Bidirectional channel for clipboard, auth, display config:
- Token-based flow control prevents blocking
- Message format: `protocol(4)|type(4)|opaque(8)|size(4)|data`
- Max packet size: 2048 bytes
- Both sides must handle unknown protocols gracefully

**Token Flow:**
1. Initial tokens from SPICE_MSG_MAIN_INIT
2. Client sends SPICE_MSGC_MAIN_AGENT_START with server tokens
3. Each side sends AGENT_TOKEN messages to allocate more
4. Cannot send more messages than available tokens

### Channel List
**SPICE_MSG_MAIN_CHANNELS_LIST**
- Sent after SPICE_MSGC_MAIN_ATTACH_CHANNELS received
- Informs client of available server channels
- Client can dynamically connect to channels

## Display Channel

### Display Control
**Operation Flow:**
1. MODE: Create display area
2. Drawing commands (between MODE and RESET)
3. MARK: Make display visible (only once)
4. RESET: Clear display and caches

**SPICE_MSG_DISPLAY_MODE**
```
UINT32 width
UINT32 height  
UINT32 depth   (16 or 32 bpp only)
```

### Client Initialization
**SPICE_MSGC_DISPLAY_INIT** (type=101)
- Enables caching and compression
- Sent once after connection
- Specifies cache sizes and compression window

### Drawing Commands
Base structure includes bounding box and clip region.

**Primitives:**
- `COPY_BITS`: Copy within display
- `DRAW_FILL`: Fill with brush pattern
- `DRAW_OPAQUE`: Blend image with brush
- `DRAW_COPY`: Copy image to display
- `DRAW_BLEND`: Mix source with destination
- `DRAW_BLACKNESS/WHITENESS`: Fill black/white
- `DRAW_INVERS`: Invert pixels
- `DRAW_ROP3`: Ternary raster operation
- `DRAW_STROKE`: Draw path with line attributes
- `DRAW_TEXT`: Render glyph string
- `DRAW_TRANSPARENT`: Copy with color key
- `DRAW_ALPHA_BLEND`: Alpha blending

### Image Formats
```
IMAGE_TYPE_PIXMAP      = 0    (raw)
SPICE_IMAGE_TYPE_QUIC  = 1    (predictive)
SPICE_IMAGE_TYPE_LZ_PLT = 100 (LZ with palette)
SPICE_IMAGE_TYPE_LZ_RGB = 101 (LZ RGB)
SPICE_IMAGE_TYPE_GLZ_RGB = 102 (global LZ)
SPICE_IMAGE_TYPE_FROM_CACHE = 103
```

### Video Streaming
**Stream Lifecycle:**
1. CREATE: Establish stream with codec
2. DATA: Send frames with timestamps  
3. CLIP: Update clipping region
4. DESTROY: Remove stream

- `STREAM_CREATE`: Create video stream
- `STREAM_DATA`: Stream frame data (timestamp + data)
- `STREAM_CLIP`: Update clipping
- `STREAM_DESTROY`: Remove stream
- `STREAM_DESTROY_ALL`: Remove all streams

**Features:**
- Frame dropping allowed
- Buffering for smooth playback
- Timestamp-based A/V sync
- Lossy compression supported

**Codec Types:**
- `STREAM_CODEC_TYPE_MJPEG = 1`

### Cache Management
**Client-side Caches:**
- Image cache (keyed by image ID)
- Palette cache (server-managed IDs)
- GLZ dictionary (cross-image compression)

**Cache Operations:**
- `IMAGE_FLAG_CACHE_ME`: Add to cache
- `IMAGE_FLAG_FROM_CACHE`: Retrieve from cache
- Explicit invalidation messages
- Reset on DISPLAY_RESET

**Invalidation:**
- `INVAL_LIST`: Remove specific resources
- `INVAL_ALL_PIXMAPS`: Clear image cache (requires sync)
- `INVAL_PALETTE`: Remove specific palette
- `INVAL_ALL_PALETTES`: Clear palette cache

## Inputs Channel

### Keyboard
- `KEY_DOWN/KEY_UP`: PC AT scan codes
- `KEY_MODIFIERS`: LED synchronization
  - Scroll Lock = 1
  - Num Lock = 2  
  - Caps Lock = 4

### Mouse
**Server Mode:** Client sends relative movements
- `MOUSE_MOTION`: Relative movement (server mode)
- `MOUSE_POSITION`: Absolute position (client mode)
- `MOUSE_PRESS/RELEASE`: Button events

**Motion Acknowledgment:**
- Server sends MOUSE_MOTION_ACK every 4 messages
- Prevents flooding, allows client to adapt rate

**Message Format:**
```
SpiceMsgcMouseMotion {
    INT32 dx, dy
    UINT32 buttons_state
}

SpiceMsgcMousePosition {
    UINT32 x, y
    UINT32 buttons_state  
    UINT8 display_id
}
```

**Button IDs:**
```
LEFT = 1, MIDDLE = 2, RIGHT = 3
UP = 4 (scroll), DOWN = 5 (scroll)
```

## Cursor Channel

### Channel Lifecycle
1. **CURSOR_INIT**: First message, sets state and clears cache
2. **Normal operation**: SET, MOVE, HIDE, etc.
3. **CURSOR_RESET**: Disable cursor and clear cache
4. After RESET, only INIT is valid

### Cursor Management
- `CURSOR_INIT`: Set initial state
- `CURSOR_SET`: Update cursor shape
- `CURSOR_MOVE`: Update position
- `CURSOR_HIDE`: Hide cursor
- `CURSOR_TRAIL`: Set cursor trail

### Cursor Types & Drawing
```
ALPHA   = 0 (ARGB8888 premultiplied)
MONO    = 1 (AND/XOR masks, 1bpp)
COLOR4  = 2 (4bpp + 16-color palette)
COLOR8  = 3 (8bpp + 256-color palette)
COLOR16 = 4 (RGB555 + mask)
COLOR24 = 5 (RGB888 + mask)
COLOR32 = 6 (RGB888 + mask)
```

**Drawing Rules (Server Mode):**
- Position cursor hot spot at mouse position
- MONO: Apply AND mask (clear bits), then XOR mask
- COLOR: Use mask, handle black/white special cases
- All non-ALPHA types include 1bpp mask

## Audio Channels

### Playback Channel (Server → Client)
**Lifecycle:**
1. MODE: Set format (required first)
2. START: Begin playback (channels, format, frequency)
3. DATA: Audio packets with timestamps
4. STOP: End playback

**Capabilities:**
- CELT_0_5_1 = 0 (obsolete)
- VOLUME = 1
- LATENCY = 2  
- OPUS = 3 (must declare support)

### Record Channel (Client → Server)
**Lifecycle:**
1. Client sends MODE (required first)
2. Server sends START
3. Client sends START_MARK (timestamp sync)
4. Client sends DATA packets
5. Server sends STOP

**Token-based Flow Control:**
- Prevents channel blocking
- Similar to agent channel

### Audio Formats
```
SPICE_AUDIO_FMT_S16 = 1 (16-bit signed PCM)
```

### Data Modes
```
RAW = 1 (PCM data)
CELT_0_5_1 = 2 (obsolete)
OPUS = 3 (compressed)
```

## Error Codes
```
SPICE_LINK_ERR_OK                    = 0
SPICE_LINK_ERR_ERROR                 = 1
SPICE_LINK_ERR_INVALID_MAGIC         = 2
SPICE_LINK_ERR_INVALID_DATA          = 3
SPICE_LINK_ERR_VERSION_MISMATCH      = 4
SPICE_LINK_ERR_NEED_SECURED          = 5
SPICE_LINK_ERR_NEED_UNSECURED        = 6
SPICE_LINK_ERR_PERMISSION_DENIED     = 7
SPICE_LINK_ERR_BAD_CONNECTION_ID     = 8
SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE = 9
```

## Security Features
- RSA-1024 for password encryption
- Time-limited tickets
- Per-channel authentication
- Optional transport encryption

## Performance Features
- **Image compression**: Quic (predictive), LZ (LZSS variant), GLZ (global dictionary)
- **Client-side caching**: Images, palettes, cursors
- **Video streaming**: Separate handling for motion video
- **Message acknowledgment**: Flow control prevents flooding
- **Multi-channel architecture**: Parallel processing
- **Dictionary compression**: GLZ maintains cross-image dictionary
- **Capabilities negotiation**: Optimize based on client support

## Implementation Notes

### Message Size Limits
- Agent packets: 2048 bytes max
- No explicit limit on other messages
- Large data uses SPICE_ADDRESS pointers

### Alignment & Padding
- Structures are packed (no padding)
- Some implementations may pad messages to 8/16 byte boundaries
- SpiceLinkReply typically ~76 bytes with padding

### Channel Relationships
- Display and Cursor channels share channel_id
- All channels share session from Main
- Channels can be added/removed dynamically

### Compatibility Rules
- Same major version = must be compatible
- Minor version increments don't break compatibility
- Bit 31 set in major = development/unsupported
- Unknown message types should be ignored