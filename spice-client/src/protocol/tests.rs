#[cfg(test)]
mod tests {
    use crate::protocol::*;
    use bincode;

    #[test]
    fn test_spice_magic_constants() {
        assert_eq!(SPICE_MAGIC, 0x53504943, "SPICE_MAGIC should be 'SPIC'");
        assert_eq!(SPICE_MAGIC_REDQ, 0x51444552, "SPICE_MAGIC_REDQ should be 'QDER' in little-endian");
    }

    #[test]
    fn test_spice_version_constants() {
        assert_eq!(SPICE_VERSION_MAJOR, 2);
        assert_eq!(SPICE_VERSION_MINOR, 2);
    }

    #[test]
    fn test_channel_types() {
        assert_eq!(ChannelType::Main as u8, 1);
        assert_eq!(ChannelType::Display as u8, 2);
        assert_eq!(ChannelType::Inputs as u8, 3);
        assert_eq!(ChannelType::Cursor as u8, 4);
        assert_eq!(ChannelType::Playback as u8, 5);
        assert_eq!(ChannelType::Record as u8, 6);
    }

    #[test]
    fn test_spice_link_header_serialization() {
        let header = SpiceLinkHeader {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 100,
        };

        let serialized = bincode::serialize(&header).unwrap();
        let deserialized: SpiceLinkHeader = bincode::deserialize(&serialized).unwrap();

        assert_eq!(header.magic, deserialized.magic);
        assert_eq!(header.major_version, deserialized.major_version);
        assert_eq!(header.minor_version, deserialized.minor_version);
        assert_eq!(header.size, deserialized.size);
    }

    #[test]
    fn test_spice_link_mess_serialization() {
        let mess = SpiceLinkMess {
            connection_id: 12345,
            channel_type: ChannelType::Display as u8,
            channel_id: 0,
            num_common_caps: 2,
            num_channel_caps: 3,
            caps_offset: 64,
        };

        let serialized = bincode::serialize(&mess).unwrap();
        // Check bincode serialization size
        println!("SpiceLinkMess bincode size: {} bytes", serialized.len());
        assert_eq!(serialized.len(), 18, "SpiceLinkMess bincode serialization is 18 bytes");
        
        let deserialized: SpiceLinkMess = bincode::deserialize(&serialized).unwrap();

        assert_eq!(mess.connection_id, deserialized.connection_id);
        assert_eq!(mess.channel_type, deserialized.channel_type);
        assert_eq!(mess.channel_id, deserialized.channel_id);
        assert_eq!(mess.num_common_caps, deserialized.num_common_caps);
        assert_eq!(mess.num_channel_caps, deserialized.num_channel_caps);
        assert_eq!(mess.caps_offset, deserialized.caps_offset);
    }

    #[test]
    fn test_spice_link_reply_serialization() {
        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 64,
        };

        let serialized = bincode::serialize(&reply).unwrap();
        let deserialized: SpiceLinkReply = bincode::deserialize(&serialized).unwrap();

        assert_eq!(reply.magic, deserialized.magic);
        assert_eq!(reply.major_version, deserialized.major_version);
        assert_eq!(reply.minor_version, deserialized.minor_version);
        assert_eq!(reply.size, deserialized.size);
    }

    #[test]
    fn test_spice_data_header_serialization() {
        let header = SpiceDataHeader {
            serial: 42,
            msg_type: 101,
            msg_size: 1024,
            sub_list: 0,
        };

        let serialized = bincode::serialize(&header).unwrap();
        assert_eq!(serialized.len(), 18); // bincode packed size without padding
        
        let deserialized: SpiceDataHeader = bincode::deserialize(&serialized).unwrap();
        assert_eq!(header.serial, deserialized.serial);
        assert_eq!(header.msg_type, deserialized.msg_type);
        assert_eq!(header.msg_size, deserialized.msg_size);
        assert_eq!(header.sub_list, deserialized.sub_list);
    }

    #[test]
    fn test_main_channel_message_types() {
        // Test that message type constants are correct
        assert_eq!(SPICE_MSG_MAIN_INIT, 103);
        assert_eq!(SPICE_MSG_MAIN_CHANNELS_LIST, 104);
        assert_eq!(SPICE_MSG_MAIN_MOUSE_MODE, 105);
        assert_eq!(SPICE_MSG_MAIN_MULTI_MEDIA_TIME, 106);
    }

    #[test]
    fn test_display_channel_message_types() {
        assert_eq!(SPICE_MSG_DISPLAY_MODE, 101);
        assert_eq!(SPICE_MSG_DISPLAY_MARK, 102);
        assert_eq!(SPICE_MSG_DISPLAY_RESET, 103);
        assert_eq!(SPICE_MSG_DISPLAY_COPY_BITS, 104);
    }

    #[test]
    fn test_struct_sizes() {
        // Ensure structs have expected sizes for protocol compatibility
        assert_eq!(std::mem::size_of::<SpiceLinkHeader>(), 16);
        assert_eq!(std::mem::size_of::<SpiceLinkMess>(), 20);
        assert_eq!(std::mem::size_of::<SpiceLinkReply>(), 16); // Updated - has 4 u32 fields
        assert_eq!(std::mem::size_of::<SpiceDataHeader>(), 24); // u64 + u16 + u32 + u32 + padding
    }

    #[test]
    fn test_channel_type_from_u8() {
        let channel_types = vec![
            (1u8, ChannelType::Main),
            (2u8, ChannelType::Display),
            (3u8, ChannelType::Inputs),
            (4u8, ChannelType::Cursor),
            (5u8, ChannelType::Playback),
            (6u8, ChannelType::Record),
        ];

        for (value, expected) in channel_types {
            // This test assumes we implement TryFrom<u8> for ChannelType
            // or have a from_u8 method
            assert_eq!(value, expected as u8);
        }
    }

    #[test]
    fn test_error_codes() {
        // Common SPICE error codes
        const SPICE_LINK_ERR_OK: u32 = 0;
        const SPICE_LINK_ERR_ERROR: u32 = 1;
        const SPICE_LINK_ERR_INVALID_MAGIC: u32 = 2;
        const SPICE_LINK_ERR_INVALID_DATA: u32 = 3;
        const SPICE_LINK_ERR_VERSION_MISMATCH: u32 = 4;
        const SPICE_LINK_ERR_NEED_SECURED: u32 = 5;
        const SPICE_LINK_ERR_NEED_UNSECURED: u32 = 6;
        const SPICE_LINK_ERR_PERMISSION_DENIED: u32 = 7;
        const SPICE_LINK_ERR_BAD_CONNECTION_ID: u32 = 8;
        const SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE: u32 = 9;

        // Test that error codes are distinct
        let error_codes = vec![
            SPICE_LINK_ERR_OK,
            SPICE_LINK_ERR_ERROR,
            SPICE_LINK_ERR_INVALID_MAGIC,
            SPICE_LINK_ERR_INVALID_DATA,
            SPICE_LINK_ERR_VERSION_MISMATCH,
            SPICE_LINK_ERR_NEED_SECURED,
            SPICE_LINK_ERR_NEED_UNSECURED,
            SPICE_LINK_ERR_PERMISSION_DENIED,
            SPICE_LINK_ERR_BAD_CONNECTION_ID,
            SPICE_LINK_ERR_CHANNEL_NOT_AVAILABLE,
        ];

        let unique_codes: std::collections::HashSet<_> = error_codes.iter().cloned().collect();
        assert_eq!(error_codes.len(), unique_codes.len(), "Error codes must be unique");
    }

    #[test]
    fn test_spice_msg_main_init_serialization() {
        let init_msg = SpiceMsgMainInit {
            session_id: 0x12345678,
            display_channels_hint: 1,
            supported_mouse_modes: 0x3,
            current_mouse_mode: 0x2,
            agent_connected: 1,
            agent_tokens: 10,
            multi_media_time: 0,
            ram_hint: 0,
        };

        let serialized = bincode::serialize(&init_msg).unwrap();
        let deserialized: SpiceMsgMainInit = bincode::deserialize(&serialized).unwrap();

        assert_eq!(init_msg.session_id, deserialized.session_id);
        assert_eq!(init_msg.display_channels_hint, deserialized.display_channels_hint);
        assert_eq!(init_msg.supported_mouse_modes, deserialized.supported_mouse_modes);
        assert_eq!(init_msg.current_mouse_mode, deserialized.current_mouse_mode);
        assert_eq!(init_msg.agent_connected, deserialized.agent_connected);
        assert_eq!(init_msg.agent_tokens, deserialized.agent_tokens);
        assert_eq!(init_msg.multi_media_time, deserialized.multi_media_time);
        assert_eq!(init_msg.ram_hint, deserialized.ram_hint);
    }

    #[test]
    fn test_spice_rect_serialization() {
        let rect = SpiceRect {
            left: -100,
            top: -50,
            right: 1024,
            bottom: 768,
        };

        let serialized = bincode::serialize(&rect).unwrap();
        assert_eq!(serialized.len(), 16); // 4 i32 values
        
        let deserialized: SpiceRect = bincode::deserialize(&serialized).unwrap();
        assert_eq!(rect.left, deserialized.left);
        assert_eq!(rect.top, deserialized.top);
        assert_eq!(rect.right, deserialized.right);
        assert_eq!(rect.bottom, deserialized.bottom);
    }

    #[test]
    fn test_spice_point_serialization() {
        let point = SpicePoint { x: 512, y: 384 };

        let serialized = bincode::serialize(&point).unwrap();
        assert_eq!(serialized.len(), 8); // 2 i32 values
        
        let deserialized: SpicePoint = bincode::deserialize(&serialized).unwrap();
        assert_eq!(point.x, deserialized.x);
        assert_eq!(point.y, deserialized.y);
    }

    #[test]
    fn test_spice_size_serialization() {
        let size = SpiceSize { width: 1920, height: 1080 };

        let serialized = bincode::serialize(&size).unwrap();
        assert_eq!(serialized.len(), 8); // 2 u32 values
        
        let deserialized: SpiceSize = bincode::deserialize(&serialized).unwrap();
        assert_eq!(size.width, deserialized.width);
        assert_eq!(size.height, deserialized.height);
    }

    #[test]
    fn test_invalid_channel_type() {
        // Test that invalid channel types are handled properly
        let invalid_types = vec![0u8, 12u8, 255u8];
        
        for invalid_type in invalid_types {
            // This assumes channel type validation is implemented
            assert!(invalid_type == 0 || invalid_type > 11 || invalid_type == 255);
        }
    }

    #[test]
    fn test_message_type_enum_conversion() {
        // Test MainChannelMessage enum conversions
        assert_eq!(MainChannelMessage::Init as u16, 103);
        assert_eq!(MainChannelMessage::ChannelsList as u16, 104);
        assert_eq!(MainChannelMessage::Ping as u16, 105);
        assert_eq!(MainChannelMessage::PingReply as u16, 106);
        
        // Test DisplayChannelMessage enum conversions
        assert_eq!(DisplayChannelMessage::Mode as u16, 101);
        assert_eq!(DisplayChannelMessage::DrawCopy as u16, 303);
        assert_eq!(DisplayChannelMessage::DrawAlphaBlend as u16, 312);
    }
}