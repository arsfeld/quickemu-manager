# SPICE-HTML5 Connection Flow

This document describes the complete connection flow of the spice-html5 client from initial connection to receiving display data.

## Sequence Diagram: Client-Server Communication

```
Client                                    SPICE Server
  |                                           |
  |-- WebSocket Connection Request ---------->|
  |<-- WebSocket Connection Established ------|
  |                                           |
  |-- SPICE Link Header -------------------->|
  |   (magic, version, etc.)                  |
  |                                           |
  |-- SpiceLinkMess ----------------------->|
  |   - connection_id                         |
  |   - channel_type (MAIN)                   |
  |   - channel_id (0)                        |
  |   - common_caps (AUTH_SELECTION,          |
  |     MINI_HEADER)                          |
  |   - channel_caps (AGENT_CONNECTED_TOKENS) |
  |                                           |
  |<-- SpiceLinkReply ------------------------|
  |    - error_code                           |
  |    - pub_key (RSA public key)             |
  |    - caps_offset, caps                    |
  |                                           |
  |-- Authentication Ticket ---------------->|
  |   (RSA encrypted password)                |
  |                                           |
  |<-- SpiceLinkAuthReply --------------------|
  |    - auth_code (OK/FAILED)                |
  |                                           |
  |                                           |
  |          === MAIN CHANNEL FLOW ===       |
  |                                           |
  |<-- SPICE_MSG_MAIN_INIT -------------------|
  |    - session_id                           |
  |    - agent_tokens                         |
  |    - mouse_mode                           |
  |    - supported_mouse_modes                |
  |                                           |
  |-- SPICE_MSGC_MAIN_ATTACH_CHANNELS ------>|
  |                                           |
  |<-- SPICE_MSG_MAIN_CHANNELS_LIST ----------|
  |    - channels[]                           |
  |      (type: DISPLAY, id: 0)               |
  |      (type: INPUTS, id: 0)                |
  |      (type: CURSOR, id: 0)                |
  |                                           |
  |                                           |
  |       === NEW DISPLAY CHANNEL ===        |
  |                                           |
  |-- WebSocket Connection Request ---------->|
  |   (new connection for display channel)    |
  |<-- WebSocket Connection Established ------|
  |                                           |
  |-- SPICE Link Header -------------------->|
  |                                           |
  |-- SpiceLinkMess ----------------------->|
  |   - connection_id (same as main)          |
  |   - channel_type (DISPLAY)                |
  |   - channel_id (0)                        |
  |   - common_caps (AUTH_SELECTION,          |
  |     MINI_HEADER)                          |
  |   - channel_caps (SIZED_STREAM,           |
  |     STREAM_REPORT, MULTI_CODEC,           |
  |     CODEC_MJPEG)                          |
  |                                           |
  |<-- SpiceLinkReply ------------------------|
  |                                           |
  |-- Authentication Ticket ---------------->|
  |                                           |
  |<-- SpiceLinkAuthReply --------------------|
  |                                           |
  |-- SPICE_MSGC_DISPLAY_INIT -------------->|
  |                                           |
  |<-- SPICE_MSG_DISPLAY_MODE ----------------|
  |    - x_res, y_res                         |
  |    - bits                                 |
  |                                           |
  |<-- SPICE_MSG_DISPLAY_SURFACE_CREATE ------|
  |    - surface_id                           |
  |    - width, height                        |
  |    - format                               |
  |                                           |
  |                                           |
  |      === DISPLAY DATA FLOW ===           |
  |                                           |
  |<-- SPICE_MSG_DISPLAY_DRAW_FILL -----------|
  |<-- SPICE_MSG_DISPLAY_DRAW_COPY -----------|
  |<-- SPICE_MSG_DISPLAY_DRAW_OPAQUE ---------|
  |<-- SPICE_MSG_DISPLAY_DRAW_BLEND ----------|
  |<-- SPICE_MSG_DISPLAY_DRAW_BLACKNESS ------|
  |<-- SPICE_MSG_DISPLAY_DRAW_WHITENESS ------|
  |<-- SPICE_MSG_DISPLAY_DRAW_INVERS ---------|
  |<-- SPICE_MSG_DISPLAY_DRAW_ROP3 -----------|
  |<-- SPICE_MSG_DISPLAY_DRAW_STROKE ---------|
  |<-- SPICE_MSG_DISPLAY_DRAW_TEXT -----------|
  |                                           |
  |<-- SPICE_MSG_DISPLAY_STREAM_CREATE -------|
  |    - id, flags                            |
  |    - codec_type (MJPEG)                   |
  |    - src_width, src_height                |
  |    - dest area                            |
  |                                           |
  |<-- SPICE_MSG_DISPLAY_STREAM_DATA ---------|
  |    - id                                   |
  |    - multimedia_time                      |
  |    - data[] (compressed video frame)      |
  |                                           |
  |-- SPICE_MSGC_DISPLAY_STREAM_REPORT ----->|
  |   - stream_id                             |
  |   - unique_id                             |
  |   - start_frame_mm_time                   |
  |   - end_frame_mm_time                     |
  |   - num_frames                            |
  |   - num_drops                             |
  |                                           |
  |                                           |
  |       === INPUT CHANNEL (Optional) ===   |
  |                                           |
  |-- SPICE_MSGC_INPUTS_KEY_DOWN ----------->|
  |-- SPICE_MSGC_INPUTS_KEY_UP ------------->|
  |-- SPICE_MSGC_INPUTS_MOUSE_MOTION ------->|
  |-- SPICE_MSGC_INPUTS_MOUSE_PRESS -------->|
  |-- SPICE_MSGC_INPUTS_MOUSE_RELEASE ------>|
  |                                           |
  |                                           |
  |       === CURSOR CHANNEL (Optional) ===  |
  |                                           |
  |<-- SPICE_MSG_CURSOR_INIT -----------------|
  |<-- SPICE_MSG_CURSOR_SET ------------------|
  |<-- SPICE_MSG_CURSOR_MOVE -----------------|
  |<-- SPICE_MSG_CURSOR_HIDE -----------------|
  |<-- SPICE_MSG_CURSOR_TRAIL ----------------|
  |                                           |
```

## ASCII Connection Flow

```
┌─────────────────────┐
│ User calls          │
│ SpiceMainConn       │
└──────────┬──────────┘
           │
           v
    ┌─────────────┐    NO   ┌─────────────────────┐
    │ WebSocket   ├────────>│ Throw Error:        │
    │ Available?  │         │ WebSocket           │
    └─────┬───────┘         │ unavailable         │
          │ YES             └─────────────────────┘
          v
┌─────────────────────┐
│ Create WebSocket    │
│ with 'binary'       │
│ protocol            │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Setup SpiceConn:    │
│ - Set connection_id │
│ - Set channel_type  │
│ - Set channel_id    │
│ - Create wire_reader│
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ WebSocket.onopen    │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Send SPICE Link     │
│ Header              │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Build SpiceLinkMess:│
│ - connection_id     │
│ - channel_type      │
│ - channel_id        │
│ - capabilities      │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Set Capabilities:   │
│ Common: PROTOCOL_   │
│ AUTH_SELECTION |    │
│ MINI_HEADER         │
│ Main: AGENT_        │
│ CONNECTED_TOKENS    │
│ Display: SIZED_     │
│ STREAM | STREAM_    │
│ REPORT | MULTI_     │
│ CODEC | CODEC_MJPEG │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Send Link Message   │
│ with Capabilities   │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Wait for            │
│ SpiceLinkReply      │
└──────────┬──────────┘
           │
           v
    ┌─────────────┐  ERROR  ┌─────────────────────┐
    │ Check Reply ├────────>│ Connection Failed   │
    │ Error Code  │         │                     │
    └─────┬───────┘         └─────────────────────┘
          │ OK
          v
┌─────────────────────┐
│ Send Authentication:│
│ RSA encrypt         │
│ password            │
└──────────┬──────────┘
           │
           v
┌─────────────────────┐
│ Wait for Auth Reply │
└──────────┬──────────┘
           │
           v
    ┌─────────────┐    NO   ┌─────────────────────┐
    │ Auth OK?    ├────────>│ Authentication      │
    │             │         │ Failed              │
    └─────┬───────┘         └─────────────────────┘
          │ YES
          v
    ┌─────────────┐
    │ Channel     │
    │ Type?       │
    └─────┬───────┘
          │
    ┌─────┴─────┬─────────────┐
    │ Display   │ Main        │
    │           │             │
    v           v             │
┌─────────────┐ ┌─────────────┐│
│ Send SPICE_ │ │ Wait for    ││
│ MSGC_       │ │ SPICE_MSG_  ││
│ DISPLAY_    │ │ MAIN_INIT   ││
│ INIT        │ │             ││
└─────┬───────┘ └─────┬───────┘│
      │               │        │
      v               v        │
┌─────────────┐ ┌─────────────┐│
│ Set state = │ │ Process     ││
│ 'ready'     │ │ Init Msg:   ││
│             │ │ - Store     ││
│             │ │   session_id││
│             │ │ - Store     ││
│             │ │   tokens    ││
│             │ │ - Check     ││
│             │ │   mouse     ││
└─────┬───────┘ └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Send SPICE_ ││
      │         │ MSGC_MAIN_  ││
      │         │ ATTACH_     ││
      │         │ CHANNELS    ││
      │         └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Wait for    ││
      │         │ SPICE_MSG_  ││
      │         │ MAIN_       ││
      │         │ CHANNELS_   ││
      │         │ LIST        ││
      │         └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Process     ││
      │         │ Channel     ││
      │         │ List:       ││
      │         │ Create      ││
      │         │ connections ││
      │         │ for each    ││
      │         │ channel     ││
      │         └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Create      ││
      │         │ Display     ││
      │         │ Channel:    ││
      │         │ New Spice   ││
      │         │ DisplayConn ││
      │         └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Display     ││
      │         │ Channel     ││
      │         │ Handshake:  ││
      │         │ Same as main││
      │         │ but with    ││
      │         │ display caps││
      │         └─────┬───────┘│
      │               │        │
      │               v        │
      │         ┌─────────────┐│
      │         │ Send SPICE_ ││
      │         │ MSGC_       ││
      │         │ DISPLAY_    ││
      │         │ INIT on     ││
      │         │ display     ││
      │         │ channel     ││
      │         └─────┬───────┘│
      │               │        │
      └───────────────┼────────┘
                      │
                      v
                ┌─────────────┐
                │ Display     │
                │ Channel     │
                │ Ready       │
                └─────┬───────┘
                      │
                      v
                ┌─────────────┐
                │ Wait for    │
                │ Display     │
                │ Messages    │
                └─────┬───────┘
                      │
                      v
                ┌─────────────┐
                │ Receive     │
                │ Message     │
                └─────┬───────┘
                      │
        ┌─────────────┼─────────────┬─────────────┬─────────────┐
        │             │             │             │             │
        v             v             v             v             v
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ SURFACE_    │ │ MODE        │ │ DRAW_*      │ │ STREAM_     │ │ Other       │
│ CREATE      │ │             │ │             │ │ CREATE      │ │ Messages    │
└─────┬───────┘ └─────┬───────┘ └─────┬───────┘ └─────┬───────┘ └─────┬───────┘
      │               │               │               │               │
      v               v               v               v               v
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ Create      │ │ Set Display │ │ Process     │ │ Create      │ │ Process     │
│ Display     │ │ Mode        │ │ Draw        │ │ Video       │ │ Other       │
│ Surface     │ │             │ │ Operations  │ │ Stream      │ │ Operations  │
└─────┬───────┘ └─────┬───────┘ └─────┬───────┘ └─────┬───────┘ └─────┬───────┘
      │               │               │               │               │
      └───────────────┼───────────────┼───────────────┼───────────────┘
                      │               │               │
                      v               │               │
                ┌─────────────┐       │               │
                │ Render to   │<──────┼───────────────┘
                │ Canvas      │<──────┘
                └─────────────┘
```

## Connection Flow Diagram

```mermaid
flowchart TD
    Start([User calls SpiceMainConn]) --> CheckWS{WebSocket<br/>Available?}
    CheckWS -->|No| Error1[Throw Error:<br/>WebSocket unavailable]
    CheckWS -->|Yes| CreateWS[Create WebSocket<br/>with 'binary' protocol]
    
    CreateWS --> SetupConn[Setup SpiceConn:<br/>- Set connection_id<br/>- Set channel_type = MAIN<br/>- Set channel_id = 0<br/>- Create wire_reader]
    
    SetupConn --> WSOpen[WebSocket.onopen]
    WSOpen --> SendHdr[Send SPICE Link Header]
    
    SendHdr --> BuildLinkMess[Build SpiceLinkMess:<br/>- connection_id<br/>- channel_type<br/>- channel_id<br/>- capabilities]
    
    BuildLinkMess --> SetCaps[Set Capabilities:<br/>Common: PROTOCOL_AUTH_SELECTION | MINI_HEADER<br/>Main: AGENT_CONNECTED_TOKENS<br/>Display: SIZED_STREAM | STREAM_REPORT | MULTI_CODEC | CODEC_MJPEG]
    
    SetCaps --> SendLink[Send Link Message<br/>with Capabilities]
    
    SendLink --> WaitReply[Wait for SpiceLinkReply]
    WaitReply --> CheckReply{Check Reply<br/>Error Code}
    
    CheckReply -->|Error| Error2[Connection Failed]
    CheckReply -->|OK| SendAuth[Send Authentication:<br/>RSA encrypt password]
    
    SendAuth --> WaitAuth[Wait for Auth Reply]
    WaitAuth --> CheckAuth{Auth OK?}
    
    CheckAuth -->|No| Error3[Authentication Failed]
    CheckAuth -->|Yes| CheckChannel{Channel Type?}
    
    CheckChannel -->|Display| SendDisplayInit[Send SPICE_MSGC_DISPLAY_INIT]
    CheckChannel -->|Main| WaitMainInit[Wait for SPICE_MSG_MAIN_INIT]
    
    SendDisplayInit --> ReadyState[Set state = 'ready']
    WaitMainInit --> ProcessInit[Process Init Message:<br/>- Store session_id<br/>- Store agent_tokens<br/>- Check mouse modes]
    
    ProcessInit --> SendAttach[Send SPICE_MSGC_MAIN_ATTACH_CHANNELS]
    SendAttach --> WaitChannels[Wait for SPICE_MSG_MAIN_CHANNELS_LIST]
    
    WaitChannels --> ProcessChannels[Process Channel List:<br/>Create connections for each channel]
    
    ProcessChannels --> CreateDisplay[Create Display Channel:<br/>New SpiceDisplayConn]
    CreateDisplay --> DisplayHandshake[Display Channel Handshake:<br/>Same as main but with display caps]
    
    DisplayHandshake --> SendDisplayInit2[Send SPICE_MSGC_DISPLAY_INIT<br/>on display channel]
    
    SendDisplayInit2 --> DisplayReady[Display Channel Ready]
    ReadyState --> DisplayReady
    
    DisplayReady --> WaitMessages[Wait for Display Messages]
    WaitMessages --> ReceiveMsg{Receive Message}
    
    ReceiveMsg -->|SURFACE_CREATE| CreateSurface[Create Display Surface]
    ReceiveMsg -->|MODE| SetMode[Set Display Mode]
    ReceiveMsg -->|DRAW_*| DrawOps[Process Draw Operations]
    ReceiveMsg -->|STREAM_CREATE| CreateStream[Create Video Stream]
    
    CreateSurface --> RenderDisplay[Render to Canvas]
    SetMode --> RenderDisplay
    DrawOps --> RenderDisplay
    CreateStream --> RenderDisplay
```

## Detailed Step-by-Step Flow

### 1. Initial Connection (main.js)
```javascript
// User creates SpiceMainConn with connection parameters
var sc = new SpiceMainConn({
    uri: 'ws://server:port',
    password: 'password'
});
```

### 2. WebSocket Setup (spiceconn.js)
```javascript
// SpiceConn constructor
this.ws = new WebSocket(o.uri, 'binary');
this.state = "connecting";
```

### 3. Link Handshake - Send (spiceconn.js:send_hdr)
```javascript
// On WebSocket open, send link header
var hdr = new SpiceLinkHeader;
hdr.magic = SPICE_MAGIC;
hdr.major_version = 2;
hdr.minor_version = 2;

var msg = new SpiceLinkMess;
msg.connection_id = this.connection_id;
msg.channel_type = this.type;
msg.channel_id = this.chan_id;
```

### 4. Capabilities Setup (spiceconn.js:143-170)
```javascript
// Common capabilities for all channels
msg.common_caps.push(
    (1 << Constants.SPICE_COMMON_CAP_PROTOCOL_AUTH_SELECTION) |
    (1 << Constants.SPICE_COMMON_CAP_MINI_HEADER)
);

// Main channel capabilities
if (msg.channel_type == Constants.SPICE_CHANNEL_MAIN) {
    msg.channel_caps.push(
        (1 << Constants.SPICE_MAIN_CAP_AGENT_CONNECTED_TOKENS)
    );
}

// Display channel capabilities
else if (msg.channel_type == Constants.SPICE_CHANNEL_DISPLAY) {
    var caps = (1 << Constants.SPICE_DISPLAY_CAP_SIZED_STREAM) |
               (1 << Constants.SPICE_DISPLAY_CAP_STREAM_REPORT) |
               (1 << Constants.SPICE_DISPLAY_CAP_MULTI_CODEC) |
               (1 << Constants.SPICE_DISPLAY_CAP_CODEC_MJPEG);
    msg.channel_caps.push(caps);
}
```

### 5. Link Reply Processing (spiceconn.js:262-278)
```javascript
// Receive SpiceLinkReply
this.reply_link = new SpiceLinkReply(mb);
if (!this.reply_link.error) {
    // Send encrypted password
    this.send_ticket(rsa_encrypt(this.reply_link.pub_key, this.password));
    this.state = "ticket";
}
```

### 6. Authentication (spiceconn.js:280-317)
```javascript
// Receive auth reply
this.auth_reply = new SpiceLinkAuthReply(mb);
if (this.auth_reply.auth_code == Constants.SPICE_LINK_ERR_OK) {
    // For display channels, send init
    if (this.type == Constants.SPICE_CHANNEL_DISPLAY) {
        var dinit = new SpiceMsgcDisplayInit();
        var reply = new SpiceMiniData();
        reply.build_msg(Constants.SPICE_MSGC_DISPLAY_INIT, dinit);
        this.send_msg(reply);
    }
    this.state = "ready";
}
```

### 7. Main Channel Initialization (main.js:95-131)
```javascript
// Process SPICE_MSG_MAIN_INIT
this.main_init = new Messages.SpiceMsgMainInit(msg.data);
this.connection_id = this.main_init.session_id;

// Send attach channels
var attach = new Messages.SpiceMiniData;
attach.type = Constants.SPICE_MSGC_MAIN_ATTACH_CHANNELS;
this.send_msg(attach);
```

### 8. Channel List Processing (main.js:147-192)
```javascript
// Process SPICE_MSG_MAIN_CHANNELS_LIST
for (i = 0; i < chans.channels.length; i++) {
    if (chans.channels[i].type == Constants.SPICE_CHANNEL_DISPLAY) {
        this.display = new SpiceDisplayConn(conn);
    }
    // Create other channels...
}
```

### 9. Display Channel Messages
Once the display channel is ready, it receives:
- `SPICE_MSG_DISPLAY_MODE` - Display configuration
- `SPICE_MSG_DISPLAY_SURFACE_CREATE` - Create rendering surface
- `SPICE_MSG_DISPLAY_DRAW_*` - Drawing operations
- `SPICE_MSG_DISPLAY_STREAM_CREATE` - Video streams

## Key Differences from Our Rust Implementation

1. **Capabilities are mandatory** - Not sending capabilities causes server to reject connection
2. **Display channels need init** - Must send `SPICE_MSGC_DISPLAY_INIT` after auth
3. **Main channel flow** - Must send `ATTACH_CHANNELS` after receiving `MAIN_INIT`
4. **Channel creation** - Server tells client which channels to create via `CHANNELS_LIST`

## Critical Success Factors

1. **Proper capability negotiation** during link phase
2. **Channel-specific initialization** after authentication
3. **Following the exact message sequence** expected by the server
4. **Handling all required message types** in the correct order