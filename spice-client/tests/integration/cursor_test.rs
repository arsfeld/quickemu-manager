use bincode;
use spice_client::channels::cursor::CursorChannel;
use spice_client::protocol::*;
use spice_client::test_utils::MockSpiceServer;
use tokio::time::Duration;

#[tokio::test]
async fn test_cursor_channel_connection() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let mut channel = CursorChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    channel.initialize().await.unwrap();

    // Verify initial state
    assert!(channel.is_cursor_visible());
    assert_eq!(channel.get_cursor_position(), (0, 0));
    assert!(channel.get_current_cursor().is_none());
}

#[tokio::test]
async fn test_cursor_set_message() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let channel = CursorChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Create cursor set message
    let mut cursor_data = Vec::new();
    cursor_data.extend_from_slice(&1234567890u64.to_le_bytes()); // unique
    cursor_data.push(0); // type
    cursor_data.extend_from_slice(&32u16.to_le_bytes()); // width
    cursor_data.extend_from_slice(&32u16.to_le_bytes()); // height
    cursor_data.extend_from_slice(&16u16.to_le_bytes()); // hot_spot_x
    cursor_data.extend_from_slice(&16u16.to_le_bytes()); // hot_spot_y

    // Add cursor pixel data (32x32 RGBA)
    let pixel_data = vec![0xFF; 32 * 32 * 4];
    cursor_data.extend_from_slice(&pixel_data);

    server
        .send_cursor_message(SPICE_MSG_CURSOR_SET, cursor_data)
        .await;

    // Process message
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify cursor was set
    let cursor = channel.get_current_cursor();
    assert!(cursor.is_some());

    if let Some(shape) = cursor {
        assert_eq!(shape.width, 32);
        assert_eq!(shape.height, 32);
        assert_eq!(shape.hot_spot_x, 16);
        assert_eq!(shape.hot_spot_y, 16);
        assert_eq!(shape.data.len(), 32 * 32 * 4);
    }
}

#[tokio::test]
async fn test_cursor_movement() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let _channel = CursorChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Send cursor move messages
    let positions = [(100, 200), (300, 400), (500, 600)];

    for (x, y) in &positions {
        let mut move_data = Vec::new();
        move_data.extend_from_slice(&(*x as i16).to_le_bytes());
        move_data.extend_from_slice(&(*y as i16).to_le_bytes());

        server
            .send_cursor_message(SPICE_MSG_CURSOR_MOVE, move_data)
            .await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Note: We can't verify position directly without processing messages
        // In a real test, we'd need to run the channel's event loop
    }
}

#[tokio::test]
async fn test_cursor_visibility() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let channel = CursorChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Initially visible
    assert!(channel.is_cursor_visible());

    // Send hide message
    let empty_data: Vec<u8> = Vec::new();
    server
        .send_cursor_message(SPICE_MSG_CURSOR_HIDE, empty_data)
        .await;

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Note: Without running the event loop, we can't verify the hide effect
}

#[tokio::test]
async fn test_cursor_cache_invalidation() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let channel = CursorChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Send cursor set messages to populate cache
    for i in 0..3 {
        let mut cursor_data = Vec::new();
        cursor_data.extend_from_slice(&(i as u64).to_le_bytes()); // unique ID
        cursor_data.push(0); // type
        cursor_data.extend_from_slice(&16u16.to_le_bytes()); // width
        cursor_data.extend_from_slice(&16u16.to_le_bytes()); // height
        cursor_data.extend_from_slice(&8u16.to_le_bytes()); // hot_spot_x
        cursor_data.extend_from_slice(&8u16.to_le_bytes()); // hot_spot_y
        cursor_data.extend(vec![0xFF; 16 * 16 * 4]); // pixel data

        server
            .send_cursor_message(SPICE_MSG_CURSOR_SET, cursor_data)
            .await;
    }

    // Send invalidate all message
    let empty_data: Vec<u8> = Vec::new();
    server
        .send_cursor_message(SPICE_MSG_CURSOR_INVAL_ALL, empty_data)
        .await;

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Note: Cache testing would require access to internal state
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_cursor_rendering() {
    use quickemu_spice_client::wasm::cursor::WasmCursorRenderer;

    let mut renderer = WasmCursorRenderer::new();

    // Create a test cursor
    let cursor = CursorShape {
        width: 16,
        height: 16,
        hot_spot_x: 8,
        hot_spot_y: 8,
        data: vec![0xFF; 16 * 16 * 4], // White cursor
        mask: None,
    };

    // Test cursor update
    let result = renderer.update_cursor(&cursor);
    assert!(result.is_ok());

    // Test cursor movement
    let result = renderer.move_cursor(100, 100);
    assert!(result.is_ok());

    // Test visibility
    let result = renderer.set_cursor_visible(false);
    assert!(result.is_ok());
}
