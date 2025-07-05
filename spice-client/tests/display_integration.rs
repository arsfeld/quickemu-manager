use spice_client::channels::display::DisplayChannel;
use spice_client::channels::Channel;
use spice_client::protocol::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_display_mode_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Complete handshake
        let mut header_buf = [0u8; 16];
        socket.read_exact(&mut header_buf).await.unwrap();

        let mut link_msg_buf = [0u8; 18];
        socket.read_exact(&mut link_msg_buf).await.unwrap();

        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 0,
        };

        let reply_bytes = bincode::serialize(&reply).unwrap();
        socket.write_all(&reply_bytes).await.unwrap();

        // Send display mode message
        let header = SpiceDataHeader {
            serial: 1,
            msg_type: SPICE_MSG_DISPLAY_MODE,
            msg_size: 20, // x, y, bits, reserved, reserved2
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();

        // Send display mode data (1024x768x32)
        let mode_data = vec![
            0x00, 0x04, 0x00, 0x00, // x = 1024
            0x00, 0x03, 0x00, 0x00, // y = 768
            0x20, 0x00, 0x00, 0x00, // bits = 32
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved2
        ];
        socket.write_all(&mode_data).await.unwrap();
    });

    let mut channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Process the mode message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, SPICE_MSG_DISPLAY_MODE);

    channel.handle_message(&header, &data).await.unwrap();

    // Check that primary surface was created
    let surface = channel.get_primary_surface();
    assert!(surface.is_some());

    if let Some(surf) = surface {
        assert_eq!(surf.width, 1024);
        assert_eq!(surf.height, 768);
        assert_eq!(surf.format, 32);
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_display_mark_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Complete handshake
        let mut header_buf = [0u8; 16];
        socket.read_exact(&mut header_buf).await.unwrap();

        let mut link_msg_buf = [0u8; 18];
        socket.read_exact(&mut link_msg_buf).await.unwrap();

        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 0,
        };

        let reply_bytes = bincode::serialize(&reply).unwrap();
        socket.write_all(&reply_bytes).await.unwrap();

        // Send display mark message
        let header = SpiceDataHeader {
            serial: 1,
            msg_type: SPICE_MSG_DISPLAY_MARK,
            msg_size: 0,
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();
    });

    let mut channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Process the mark message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, SPICE_MSG_DISPLAY_MARK);
    assert_eq!(data.len(), 0);

    // Should not error
    channel.handle_message(&header, &data).await.unwrap();

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_display_surface_create_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Complete handshake
        let mut header_buf = [0u8; 16];
        socket.read_exact(&mut header_buf).await.unwrap();

        let mut link_msg_buf = [0u8; 18];
        socket.read_exact(&mut link_msg_buf).await.unwrap();

        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 0,
        };

        let reply_bytes = bincode::serialize(&reply).unwrap();
        socket.write_all(&reply_bytes).await.unwrap();

        // Send surface create message
        let header = SpiceDataHeader {
            serial: 1,
            msg_type: SPICE_MSG_DISPLAY_SURFACE_CREATE,
            msg_size: 24, // surface_id, width, height, format, flags, reserved
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();

        // Send surface create data
        let surface_data = vec![
            0x00, 0x00, 0x00, 0x00, // surface_id = 0
            0x80, 0x07, 0x00, 0x00, // width = 1920
            0x38, 0x04, 0x00, 0x00, // height = 1080
            0x20, 0x00, 0x00, 0x00, // format = 32 (ARGB)
            0x00, 0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // reserved
        ];
        socket.write_all(&surface_data).await.unwrap();
    });

    let mut channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Process the surface create message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, SPICE_MSG_DISPLAY_SURFACE_CREATE);

    channel.handle_message(&header, &data).await.unwrap();

    // Check that surface was created
    let surface = channel.get_primary_surface();
    assert!(surface.is_some());

    if let Some(surf) = surface {
        assert_eq!(surf.width, 1920);
        assert_eq!(surf.height, 1080);
        assert_eq!(surf.format, 32);
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_display_draw_copy_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Complete handshake
        let mut header_buf = [0u8; 16];
        socket.read_exact(&mut header_buf).await.unwrap();

        let mut link_msg_buf = [0u8; 18];
        socket.read_exact(&mut link_msg_buf).await.unwrap();

        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 0,
        };

        let reply_bytes = bincode::serialize(&reply).unwrap();
        socket.write_all(&reply_bytes).await.unwrap();

        // First send display mode to create surface
        let mode_header = SpiceDataHeader {
            serial: 1,
            msg_type: SPICE_MSG_DISPLAY_MODE,
            msg_size: 20,
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&mode_header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();

        let mode_data = vec![
            0x00, 0x04, 0x00, 0x00, // x = 1024
            0x00, 0x03, 0x00, 0x00, // y = 768
            0x20, 0x00, 0x00, 0x00, // bits = 32
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved2
        ];
        socket.write_all(&mode_data).await.unwrap();

        // Then send draw copy message
        let draw_header = SpiceDataHeader {
            serial: 2,
            msg_type: DisplayChannelMessage::DrawCopy as u16,
            msg_size: 32, // Simplified draw copy data
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&draw_header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();

        // Send minimal draw copy data
        let draw_data = vec![0u8; 32];
        socket.write_all(&draw_data).await.unwrap();
    });

    let mut channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Process the mode message first
    let (header, data) = channel.connection.read_message().await.unwrap();
    channel.handle_message(&header, &data).await.unwrap();

    // Process the draw copy message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, DisplayChannelMessage::DrawCopy as u16);

    // Should handle without error (even if not fully implemented)
    channel.handle_message(&header, &data).await.unwrap();

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_display_stream_create_destroy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        // Complete handshake
        let mut header_buf = [0u8; 16];
        socket.read_exact(&mut header_buf).await.unwrap();

        let mut link_msg_buf = [0u8; 18];
        socket.read_exact(&mut link_msg_buf).await.unwrap();

        let reply = SpiceLinkReply {
            magic: SPICE_MAGIC,
            major_version: SPICE_VERSION_MAJOR,
            minor_version: SPICE_VERSION_MINOR,
            size: 0,
        };

        let reply_bytes = bincode::serialize(&reply).unwrap();
        socket.write_all(&reply_bytes).await.unwrap();

        // Send stream create message
        let create_header = SpiceDataHeader {
            serial: 1,
            msg_type: DisplayChannelMessage::StreamCreate as u16,
            msg_size: 4, // stream_id
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&create_header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();
        socket.write_all(&[0x01, 0x00, 0x00, 0x00]).await.unwrap(); // stream_id = 1

        // Send stream destroy message
        let destroy_header = SpiceDataHeader {
            serial: 2,
            msg_type: DisplayChannelMessage::StreamDestroy as u16,
            msg_size: 4, // stream_id
            sub_list: 0,
        };

        let header_bytes = bincode::serialize(&destroy_header).unwrap();
        socket.write_all(&header_bytes).await.unwrap();
        socket.write_all(&[0x01, 0x00, 0x00, 0x00]).await.unwrap(); // stream_id = 1
    });

    let mut channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Process stream create message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, DisplayChannelMessage::StreamCreate as u16);
    channel.handle_message(&header, &data).await.unwrap();

    // Process stream destroy message
    let (header, data) = channel.connection.read_message().await.unwrap();
    assert_eq!(header.msg_type, DisplayChannelMessage::StreamDestroy as u16);
    channel.handle_message(&header, &data).await.unwrap();

    server_task.await.unwrap();
}
