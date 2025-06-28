use spice_client::channels::display::DisplayChannel;
use spice_client::protocol::*;
use spice_client::test_utils::MockSpiceServer;
use tokio::time::Duration;

#[tokio::test]
async fn test_multi_display_support() {
    // Start mock server
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();
    
    // Create display channel
    let channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();
    
    // Test creating multiple surfaces
    let surface1 = SpiceSurfaceCreate {
        surface_id: 0,
        width: 1920,
        height: 1080,
        format: 32, // SPICE_SURFACE_FMT_32_xRGB
        flags: 0,
    };
    
    let surface2 = SpiceSurfaceCreate {
        surface_id: 1,
        width: 1920,
        height: 1080,
        format: 32,
        flags: 0,
    };
    
    // Send surface create messages
    server.send_display_message(SPICE_MSG_DISPLAY_SURFACE_CREATE, &surface1).await;
    server.send_display_message(SPICE_MSG_DISPLAY_SURFACE_CREATE, &surface2).await;
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify surfaces were created
    assert!(channel.get_surface(0).is_some());
    assert!(channel.get_surface(1).is_some());
    
    let surface0 = channel.get_surface(0).unwrap();
    assert_eq!(surface0.width, 1920);
    assert_eq!(surface0.height, 1080);
    
    let surface1 = channel.get_surface(1).unwrap();
    assert_eq!(surface1.width, 1920);
    assert_eq!(surface1.height, 1080);
}

#[tokio::test]
async fn test_monitors_config() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();
    
    let channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();
    
    // Create monitors configuration
    let monitors_config = SpiceMonitorsConfig {
        count: 2,
        max_allowed: 4,
        heads: vec![
            SpiceHead {
                id: 0,
                surface_id: 0,
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                flags: SPICE_HEAD_FLAGS_PRIMARY,
            },
            SpiceHead {
                id: 1,
                surface_id: 1,
                width: 1920,
                height: 1080,
                x: 1920,
                y: 0,
                flags: SPICE_HEAD_FLAGS_NONE,
            },
        ],
    };
    
    // Send monitors config message
    server.send_display_message(SPICE_MSG_DISPLAY_MONITORS_CONFIG, &monitors_config).await;
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify monitors configuration
    let monitors = channel.get_monitors();
    assert_eq!(monitors.len(), 2);
    
    assert_eq!(monitors[0].width, 1920);
    assert_eq!(monitors[0].height, 1080);
    assert_eq!(monitors[0].x, 0);
    assert_eq!(monitors[0].flags, SPICE_HEAD_FLAGS_PRIMARY);
    
    assert_eq!(monitors[1].width, 1920);
    assert_eq!(monitors[1].height, 1080);
    assert_eq!(monitors[1].x, 1920);
    assert_eq!(monitors[1].flags, SPICE_HEAD_FLAGS_NONE);
}

#[tokio::test]
async fn test_multi_display_video_streams() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();
    
    let channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();
    
    // Create two surfaces first
    let surface1 = SpiceSurfaceCreate {
        surface_id: 0,
        width: 1920,
        height: 1080,
        format: 32,
        flags: 0,
    };
    
    let surface2 = SpiceSurfaceCreate {
        surface_id: 1,
        width: 1920,
        height: 1080,
        format: 32,
        flags: 0,
    };
    
    server.send_display_message(SPICE_MSG_DISPLAY_SURFACE_CREATE, &surface1).await;
    server.send_display_message(SPICE_MSG_DISPLAY_SURFACE_CREATE, &surface2).await;
    
    // Create video streams on different surfaces
    let stream1 = SpiceStreamCreate {
        id: 0,
        flags: 0,
        codec_type: 1, // MJPEG
        stamp: 0,
        stream_width: 1280,
        stream_height: 720,
        src_width: 1280,
        src_height: 720,
        dest: SpiceRect {
            left: 100,
            top: 100,
            right: 1380,
            bottom: 820,
        },
        clip: SpiceClip {
            clip_type: 0, // NONE
            data: None,
        },
    };
    
    let stream2 = SpiceStreamCreate {
        id: 1,
        flags: 0,
        codec_type: 1, // MJPEG
        stamp: 0,
        stream_width: 1280,
        stream_height: 720,
        src_width: 1280,
        src_height: 720,
        dest: SpiceRect {
            left: 100,
            top: 100,
            right: 1380,
            bottom: 820,
        },
        clip: SpiceClip {
            clip_type: 0,
            data: None,
        },
    };
    
    server.send_display_message(DisplayChannelMessage::StreamCreate as u16, &stream1).await;
    server.send_display_message(DisplayChannelMessage::StreamCreate as u16, &stream2).await;
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // TODO: Add assertions once we have access to active_streams
    // For now, just verify no panic occurred
}

#[tokio::test]
async fn test_surface_cleanup_on_reset() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();
    
    let channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();
    
    // Create multiple surfaces
    for i in 0..3 {
        let surface = SpiceSurfaceCreate {
            surface_id: i,
            width: 1920,
            height: 1080,
            format: 32,
            flags: 0,
        };
        server.send_display_message(SPICE_MSG_DISPLAY_SURFACE_CREATE, &surface).await;
    }
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify surfaces exist
    assert!(channel.get_surface(0).is_some());
    assert!(channel.get_surface(1).is_some());
    assert!(channel.get_surface(2).is_some());
    
    // Send reset message
    server.send_display_message(DisplayChannelMessage::Reset as u16, &[]).await;
    
    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify all surfaces were cleared
    assert!(channel.get_surface(0).is_none());
    assert!(channel.get_surface(1).is_none());
    assert!(channel.get_surface(2).is_none());
    assert_eq!(channel.get_monitors().len(), 0);
}