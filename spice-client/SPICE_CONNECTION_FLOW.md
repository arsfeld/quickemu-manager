# SPICE Connection Protocol Flow

This document describes the complete connection flow between a SPICE client and server, including all messages exchanged until the display channel issue occurs.

## Connection Flow Diagram

```
┌─────────────┐                                              ┌─────────────┐
│   CLIENT    │                                              │   SERVER    │
└─────────────┘                                              └─────────────┘
      │                                                              │
      │                    1. TCP CONNECTION                         │
      ├─────────────────────────────────────────────────────────────►
      │                   Connect to port 5900                       │
      │                                                              │
      │                  2. SPICE LINK PROTOCOL                      │
      │                                                              │
      │  SpiceLinkHeader                                             │
      ├─────────────────────────────────────────────────────────────►
      │  - magic: 0x51444552 ("REDQ")                                │
      │  - major_version: 2                                          │
      │  - minor_version: 2                                          │
      │  - size: 20                                                  │
      │                                                              │
      │  SpiceLinkMess                                               │
      ├─────────────────────────────────────────────────────────────►
      │  - connection_id: 0 (new session)                            │
      │  - channel_type: 1 (Main)                                    │
      │  - channel_id: 0                                              │
      │  - num_common_caps: 1                                         │
      │  - num_channel_caps: 0                                        │
      │  - caps_offset: 20                                            │
      │                                                              │
      │                    SpiceLinkReply                             │
      ◄─────────────────────────────────────────────────────────────┤
      │  - magic: 0x51444552                                          │
      │  - major_version: 2                                            │
      │  - minor_version: 2                                            │
      │  - size: 186                                                   │
      │                                                              │
      │                    SpiceLinkReplyData                         │
      ◄─────────────────────────────────────────────────────────────┤
      │  - error: 0 (SPICE_LINK_ERR_OK)                              │
      │  - pub_key: [RSA 1024-bit public key, 162 bytes]             │
      │  - num_common_caps: 1                                         │
      │  - num_channel_caps: 11                                       │
      │  - caps_offset: 178                                           │
      │                                                              │
      │  3. AUTHENTICATION (if required)                              │
      │                                                              │
      │  Encrypted password (128 bytes)                               │
      ├─────────────────────────────────────────────────────────────►
      │  (RSA encrypted with server's public key)                     │
      │                                                              │
      │                    Link Result                                │
      ◄─────────────────────────────────────────────────────────────┤
      │  4 bytes: 0x00000000 (SPICE_LINK_ERR_OK)                     │
      │                                                              │
      │              4. MAIN CHANNEL INITIALIZATION                   │
      │                                                              │
      │                    SPICE_MSG_MAIN_INIT                        │
      ◄─────────────────────────────────────────────────────────────┤
      │  Header: serial=1, type=103, size=32                          │
      │  - session_id: 0x57fd1f78 (unique per connection)            │
      │  - display_channels_hint: 1                                  │
      │  - supported_mouse_modes: 3                                  │
      │  - current_mouse_mode: 1                                     │
      │  - agent_connected: 0                                         │
      │  - agent_tokens: 10                                           │
      │  - multi_media_time: 0x24531689                              │
      │  - ram_hint: 0x01000000                                       │
      │                                                              │
      │  SPICE_MSGC_MAIN_ATTACH_CHANNELS                             │
      ├─────────────────────────────────────────────────────────────►
      │  Header: serial=1, type=104, size=0                           │
      │  (Empty message - tells server to activate channels)          │
      │                                                              │
      │                    SPICE_MSG_PING                             │
      ◄─────────────────────────────────────────────────────────────┤
      │  Header: serial=2, type=4, size=12                            │
      │  - id: 1                                                      │
      │  - timestamp: 0xe496228b                                      │
      │  - extra_data: 0x0000008d (141)                               │
      │                                                              │
      │  SPICE_MSGC_PONG                                              │
      ├─────────────────────────────────────────────────────────────►
      │  Header: serial=1, type=3, size=12                            │
      │  - id: 1                                                      │
      │  - timestamp: 0xe496228b                                      │
      │  - extra_data: 0x0000008d                                     │
      │                                                              │
      │              5. CHANNEL LIST DISCOVERY                        │
      │                                                              │
      │                    SPICE_MSG_MAIN_CHANNELS_LIST               │
      ◄─────────────────────────────────────────────────────────────┤
      │  Header: serial=6, type=104, size=16                          │
      │  - num_channels: 4                                            │
      │  - channels[0]: type=2 (Display), id=0                       │
      │  - channels[1]: type=3 (Inputs), id=0                        │
      │  - channels[2]: type=4 (Cursor), id=0                        │
      │  - channels[3]: type=5 (Playback), id=0                      │
      │                                                              │
      │           6. SECONDARY CHANNEL CONNECTIONS                    │
      │                                                              │
      │  ┌─────────────── Display Channel (type=2, id=0) ───────────┐ │
      │  │                                                          │ │
      │  │  New TCP Connection                                      │ │
      ├──┼──────────────────────────────────────────────────────────┼─►
      │  │                                                          │ │
      │  │  SpiceLinkHeader & SpiceLinkMess                        │ │
      ├──┼──────────────────────────────────────────────────────────┼─►
      │  │  - connection_id: 0x57fd1f78 (session_id from main)     │ │
      │  │  - channel_type: 2 (Display)                            │ │
      │  │  - channel_id: 0                                        │ │
      │  │                                                          │ │
      │  │                    SpiceLinkReply                       │ │
      ◄──┼──────────────────────────────────────────────────────────┼─┤
      │  │  (Same as main channel)                                  │ │
      │  │                                                          │ │
      │  │  Encrypted password                                      │ │
      ├──┼──────────────────────────────────────────────────────────┼─►
      │  │                                                          │ │
      │  │                    Link Result: OK                      │ │
      ◄──┼──────────────────────────────────────────────────────────┼─┤
      │  │                                                          │ │
      │  │              DISPLAY CHANNEL READY                       │ │
      │  │         (Now waiting for display messages)              │ │
      │  └─────────────────────────────────────────────────────────┘ │
      │                                                              │
      │  ┌─────────────── Inputs Channel (type=3, id=0) ────────────┐ │
      │  │  (Similar connection flow as Display)                    │ │
      │  └─────────────────────────────────────────────────────────┘ │
      │                                                              │
      │  ┌─────────────── Cursor Channel (type=4, id=0) ────────────┐ │
      │  │  (Similar connection flow as Display)                    │ │
      │  └─────────────────────────────────────────────────────────┘ │
      │                                                              │
      │              7. DISPLAY CHANNEL ISSUE                        │
      │                                                              │
      │  Display channel connects successfully but...                 │
      │                                                              │
      │  Expected: SPICE_MSG_DISPLAY_MODE                            │
      │           SPICE_MSG_DISPLAY_SURFACE_CREATE                   │
      │           SPICE_MSG_DISPLAY_DRAW_*                           │
      ◄ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┤
      │                                                              │
      │  Actual: No messages received                                │
      │          Channel's read_message() blocks forever             │
      │                                                              │
      │                    ❌ TIMEOUT ❌                              │
      │                                                              │
```

## Message Details

### 1. Initial Connection
- Client connects to server on TCP port 5900
- Server accepts connection

### 2. SPICE Link Protocol
- Client sends `SpiceLinkHeader` + `SpiceLinkMess` (36 bytes total)
- Server responds with `SpiceLinkReply` + `SpiceLinkReplyData` + capabilities
- Contains RSA public key for authentication

### 3. Authentication
- Client encrypts password (or empty string) with server's RSA key
- Sends 128-byte encrypted data
- Server responds with 4-byte result (0 = success)

### 4. Main Channel Initialization
- Server sends `SPICE_MSG_MAIN_INIT` with session details
- **Critical**: Client sends `SPICE_MSGC_MAIN_ATTACH_CHANNELS` 
  - This activates all channels on the server
  - Must be sent after main channel init but before secondary channels connect
- Server starts sending PING messages for latency measurement

### 5. Channel Discovery
- Server sends `SPICE_MSG_MAIN_CHANNELS_LIST`
- Lists all available channels (Display, Inputs, Cursor, etc.)

### 6. Secondary Channel Connections
- For each channel in the list:
  - Client opens new TCP connection
  - Performs same link protocol but with session_id as connection_id
  - Each channel gets authenticated separately

### 7. The Display Channel Issue

**Expected behavior:**
- After display channel connects, server should send:
  - `SPICE_MSG_DISPLAY_MODE` - Display configuration
  - `SPICE_MSG_DISPLAY_SURFACE_CREATE` - Create drawing surface
  - Various draw commands (`SPICE_MSG_DISPLAY_DRAW_FILL`, etc.)

**Actual behavior:**
- Display channel connects successfully
- Authentication succeeds
- But no messages are received
- `read_message()` blocks waiting for data that never comes

## Possible Causes

1. **Missing Initialization**: Display channel might need to send an init message first
2. **Capability Negotiation**: Server might be waiting for capability advertisement
3. **ATTACH_CHANNELS Timing**: The timing of when ATTACH_CHANNELS is sent matters
4. **VM Display State**: The QEMU VM might not have display output configured

## Key Findings

- Each SPICE channel uses a separate TCP connection
- The `session_id` from main channel is used as `connection_id` for secondary channels
- `SPICE_MSGC_MAIN_ATTACH_CHANNELS` is crucial for activating channels
- The display channel connects but receives no data, suggesting a protocol issue