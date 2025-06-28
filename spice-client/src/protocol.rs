use serde::{Deserialize, Serialize};

pub const SPICE_MAGIC: u32 = 0x53504943; // "SPIC"
pub const SPICE_MAGIC_REDQ: u32 = 0x51444552; // "REDQ" - alternative magic seen in some implementations
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SpiceRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpicePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests;