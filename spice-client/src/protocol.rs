use serde::{Deserialize, Serialize};

pub const SPICE_MAGIC: u32 = 0x51444552; // "REDQ" - official SPICE protocol magic
pub const SPICE_MAGIC_LEGACY: u32 = 0x53504943; // "SPIC" - legacy/alternative magic
pub const SPICE_VERSION_MAJOR: u32 = 2;
pub const SPICE_VERSION_MINOR: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelType {
    Main = 1,
    Display = 2,
    Inputs = 3,
    Cursor = 4,
    Playback = 5,
    Record = 6,
    Tunnel = 7,
    SmartCard = 8,
    UsbreDirect = 9,
    Port = 10,
    WebDav = 11,
}

impl From<u8> for ChannelType {
    fn from(value: u8) -> Self {
        match value {
            1 => ChannelType::Main,
            2 => ChannelType::Display,
            3 => ChannelType::Inputs,
            4 => ChannelType::Cursor,
            5 => ChannelType::Playback,
            6 => ChannelType::Record,
            7 => ChannelType::Tunnel,
            8 => ChannelType::SmartCard,
            9 => ChannelType::UsbreDirect,
            10 => ChannelType::Port,
            11 => ChannelType::WebDav,
            _ => ChannelType::Main, // Default fallback
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceDataHeader {
    pub serial: u64,
    pub msg_type: u16,
    pub msg_size: u32,
    pub sub_list: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceLinkHeader {
    pub magic: u32,
    pub major_version: u32,
    pub minor_version: u32,
    pub size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceLinkMess {
    pub connection_id: u32,
    pub channel_type: u8,
    pub channel_id: u8,
    pub num_common_caps: u32,
    pub num_channel_caps: u32,
    pub caps_offset: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceLinkReply {
    pub magic: u32,
    pub major_version: u32,
    pub minor_version: u32,
    pub size: u32,
}

#[derive(Debug)]
pub struct SpiceLinkReplyData {
    pub error: u32,
    pub pub_key: Vec<u8>,  // RSA 1024-bit public key (162 bytes)
    pub num_common_caps: u32,
    pub num_channel_caps: u32,
    pub caps_offset: u32,
}

// Main channel messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MainChannelMessage {
    MouseMode = 101,
    MouseModeReply = 102,
    Init = 103,
    ChannelsList = 104,
    Ping = 105,
    PingReply = 106,
    Notify = 107,
    Disconnecting = 108,
    MultiMediaTime = 109,
    AgentConnected = 110,
    AgentDisconnected = 111,
    AgentData = 112,
    AgentTokens = 113,
}

// Display channel messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DisplayChannelMessage {
    Mode = 101,
    Mark = 102,
    Reset = 103,
    CopyBits = 104,
    InvalList = 105,
    InvalAllPixmaps = 106,
    InvalAllPalettes = 107,
    StreamCreate = 108,
    StreamData = 109,
    StreamClip = 110,
    StreamDestroy = 111,
    StreamDestroyAll = 112,
    DrawFill = 301,
    DrawOpaque = 302,
    DrawCopy = 303,
    DrawBlend = 304,
    DrawBlackness = 305,
    DrawWhiteness = 306,
    DrawInvers = 307,
    DrawRop3 = 308,
    DrawStroke = 309,
    DrawText = 310,
    DrawTransparent = 311,
    DrawAlphaBlend = 312,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpicePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceSize {
    pub width: u32,
    pub height: u32,
}

// Message type constants
pub const SPICE_MSG_MAIN_INIT: u16 = 103;
pub const SPICE_MSG_MAIN_CHANNELS_LIST: u16 = 104;
pub const SPICE_MSG_MAIN_MOUSE_MODE: u16 = 105;
pub const SPICE_MSG_MAIN_MULTI_MEDIA_TIME: u16 = 106;

pub const SPICE_MSG_DISPLAY_MODE: u16 = 101;
pub const SPICE_MSG_DISPLAY_MARK: u16 = 102;
pub const SPICE_MSG_DISPLAY_RESET: u16 = 103;
pub const SPICE_MSG_DISPLAY_COPY_BITS: u16 = 104;
pub const SPICE_MSG_DISPLAY_DRAW_ALPHA_BLEND: u16 = 317;
pub const SPICE_MSG_DISPLAY_SURFACE_CREATE: u16 = 318;
pub const SPICE_MSG_DISPLAY_SURFACE_DESTROY: u16 = 319;
pub const SPICE_MSG_DISPLAY_MONITORS_CONFIG: u16 = 320;
pub const SPICE_MSG_DISPLAY_DRAW_COMPOSITE: u16 = 321;

// Cursor channel messages
pub const SPICE_MSG_CURSOR_INIT: u16 = 101;
pub const SPICE_MSG_CURSOR_RESET: u16 = 102;
pub const SPICE_MSG_CURSOR_SET: u16 = 103;
pub const SPICE_MSG_CURSOR_MOVE: u16 = 104;
pub const SPICE_MSG_CURSOR_HIDE: u16 = 105;
pub const SPICE_MSG_CURSOR_TRAIL: u16 = 106;
pub const SPICE_MSG_CURSOR_INVAL_ONE: u16 = 107;
pub const SPICE_MSG_CURSOR_INVAL_ALL: u16 = 108;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceMsgMainInit {
    pub session_id: u32,
    pub display_channels_hint: u32,
    pub supported_mouse_modes: u32,
    pub current_mouse_mode: u32,
    pub agent_connected: u32,
    pub agent_tokens: u32,
    pub multi_media_time: u32,
    pub ram_hint: u32,
}

// Main channel structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainMouseMode {
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainMultiMediaTime {
    pub time: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainAgentConnected {
    pub error_code: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainAgentData {
    pub protocol: u32,
    pub type_: u32,
    pub opaque: u64,
    pub size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainNotify {
    pub time_stamp: u64,
    pub severity: u32,
    pub visibility: u32,
    pub what: u32,
    pub message_len: u32,
    pub message: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMsgMainAgentTokens {
    pub num_tokens: u32,
}

// Display drawing structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceBrush {
    pub brush_type: u8,
    pub color: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceClip {
    pub clip_type: u8,
    pub data: Option<SpiceRect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceDrawBase {
    pub surface_id: u32,
    pub box_rect: SpiceRect,
    pub clip: SpiceClip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceDrawFill {
    pub base: SpiceDrawBase,
    pub brush: SpiceBrush,
    pub rop_descriptor: u16,
    pub mask: Option<SpiceQMask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceQMask {
    pub flags: u8,
    pub pos: SpicePoint,
    pub bitmap: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceDrawCopy {
    pub base: SpiceDrawBase,
    pub src_area: SpiceRect,
    pub rop_descriptor: u16,
    pub scale_mode: u8,
    pub mask: Option<SpiceQMask>,
    pub src_image: Option<Vec<u8>>, // Simplified: actual SPICE uses complex image structures
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceDrawOpaque {
    pub base: SpiceDrawBase,
    pub src_area: SpiceRect,
    pub brush: SpiceBrush,
    pub rop_descriptor: u16,
    pub scale_mode: u8,
    pub mask: Option<SpiceQMask>,
    pub src_image: Option<Vec<u8>>, // Simplified: actual SPICE uses complex image structures
}

// Stream structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceStreamCreate {
    pub id: u32,
    pub flags: u8,
    pub codec_type: u8,
    pub stamp: u64,
    pub stream_width: u32,
    pub stream_height: u32,
    pub src_width: u32,
    pub src_height: u32,
    pub dest: SpiceRect,
    pub clip: SpiceClip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceStreamData {
    pub id: u32,
    pub multi_media_time: u32,
    pub data_size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceStreamDestroy {
    pub id: u32,
}

// Surface structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceSurfaceCreate {
    pub surface_id: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceSurfaceDestroy {
    pub surface_id: u32,
}

// Multi-display support structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceHead {
    pub id: u32,
    pub surface_id: u32,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceMonitorsConfig {
    pub count: u16,
    pub max_allowed: u16,
    pub heads: Vec<SpiceHead>,
}

// Monitor flags
pub const SPICE_HEAD_FLAGS_NONE: u32 = 0;
pub const SPICE_HEAD_FLAGS_PRIMARY: u32 = 1 << 0;

#[cfg(test)]
mod tests;