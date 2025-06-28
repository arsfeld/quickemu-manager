# SPICE Protocol Definition v2.2

## Overview
SPICE (Simple Protocol for Independent Computing Environments) provides remote access to computing devices across networks. It uses simple messaging without RPC dependencies and supports multiple communication channels for different device types.

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

## Connection Process

### 1. Link Stage
**Client → Server: SpiceLinkMess**
```
UINT32 magic            = SPICE_MAGIC
UINT32 major_version    = SPICE_VERSION_MAJOR
UINT32 minor_version    = SPICE_VERSION_MINOR
UINT32 size            (bytes following)
UINT32 connection_id   (0 for main channel)
UINT8  channel_type    
UINT8  channel_id      
UINT32 num_common_caps 
UINT32 num_channel_caps
UINT32 caps_offset     
```

**Server → Client: SpiceLinkReply**
```
UINT32 magic
UINT32 major_version
UINT32 minor_version
UINT32 size
UINT32 error
UINT8[162] pub_key     (RSA 1024-bit)
UINT32 num_common_caps
UINT32 num_channel_caps
UINT32 caps_offset
```

**Client → Server: Encrypted Password**
- RSA-OAEP with SHA-1, MGF1

**Server → Client: Link Result**
- UINT32 error code

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
**SPICE_MSG_MAIN_INIT**
```
UINT32 session_id
UINT32 display_channels_hint
UINT32 supported_mouse_modes
UINT32 current_mouse_mode
UINT32 agent_connected
UINT32 agent_tokens
UINT32 multi_media_time
UINT32 ram_hint
```

### Mouse Modes
- `SPICE_MOUSE_MODE_SERVER = 1`: Server controls cursor
- `SPICE_MOUSE_MODE_CLIENT = 2`: Client controls cursor

### Migration Control
- `SPICE_MSG_MAIN_MIGRATE_BEGIN`: Start migration
- `SPICE_MSG_MAIN_MIGRATE_CANCEL`: Cancel migration
- `SPICE_MSGC_MAIN_MIGRATE_CONNECTED`: Migration ready
- `SPICE_MSGC_MAIN_MIGRATE_CONNECT_ERROR`: Migration failed

### Agent Communication
Bidirectional channel for clipboard, auth, etc.
- Token-based flow control
- Message format: `protocol|type|opaque|size|data`
- Max packet size: 2048 bytes

## Display Channel

### Display Control
- `SPICE_MSG_DISPLAY_MODE`: Set display area (width, height, depth)
- `SPICE_MSG_DISPLAY_MARK`: Begin visibility
- `SPICE_MSG_DISPLAY_RESET`: Clear display and caches

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
- `STREAM_CREATE`: Create video stream
- `STREAM_DATA`: Stream frame data
- `STREAM_CLIP`: Update clipping
- `STREAM_DESTROY`: Remove stream

**Codec Types:**
- `STREAM_CODEC_TYPE_MJPEG = 1`

### Cache Management
- Image cache (client-side)
- Palette cache (server-managed)
- Invalidation messages for cache control

## Inputs Channel

### Keyboard
- `KEY_DOWN/KEY_UP`: PC AT scan codes
- `KEY_MODIFIERS`: LED synchronization
  - Scroll Lock = 1
  - Num Lock = 2  
  - Caps Lock = 4

### Mouse
- `MOUSE_MOTION`: Relative movement (server mode)
- `MOUSE_POSITION`: Absolute position (client mode)
- `MOUSE_PRESS/RELEASE`: Button events
- Motion acknowledgment every 4 messages

**Button IDs:**
```
LEFT = 1, MIDDLE = 2, RIGHT = 3
UP = 4 (scroll), DOWN = 5 (scroll)
```

## Cursor Channel

### Cursor Management
- `CURSOR_INIT`: Set initial state
- `CURSOR_SET`: Update cursor shape
- `CURSOR_MOVE`: Update position
- `CURSOR_HIDE`: Hide cursor
- `CURSOR_TRAIL`: Set cursor trail

### Cursor Types
```
ALPHA   = 0 (ARGB8888)
MONO    = 1 (AND/XOR masks)
COLOR4  = 2 (4bpp + palette)
COLOR8  = 3 (8bpp + palette)
COLOR16 = 4 (RGB555)
COLOR24 = 5 (RGB888)
COLOR32 = 6 (RGB888)
```

## Audio Channels

### Playback Channel
- `PLAYBACK_MODE`: Set data format
- `PLAYBACK_START`: Begin playback
- `PLAYBACK_DATA`: Audio packets
- `PLAYBACK_STOP`: End playback

### Record Channel
- `RECORD_START`: Begin capture
- `RECORD_DATA`: Audio packets
- `RECORD_STOP`: End capture
- `RECORD_START_MARK`: Timestamp sync

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
- Image compression (Quic, LZ, GLZ)
- Client-side caching
- Video streaming for motion
- Message acknowledgment flow control
- Multi-channel architecture
- Dictionary-based compression