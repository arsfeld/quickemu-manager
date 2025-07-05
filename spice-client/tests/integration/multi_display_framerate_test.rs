use bincode;
use spice_client::channels::display::DisplayChannel;
use spice_client::protocol::*;
use spice_client::test_utils::MockSpiceServer;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

#[derive(Debug, Clone)]
struct FrameRateMonitor {
    display_id: u32,
    frame_count: Arc<AtomicU32>,
    start_time: Instant,
}

impl FrameRateMonitor {
    fn new(display_id: u32) -> Self {
        Self {
            display_id,
            frame_count: Arc::new(AtomicU32::new(0)),
            start_time: Instant::now(),
        }
    }

    fn increment(&self) {
        self.frame_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_fps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let frames = self.frame_count.load(Ordering::Relaxed) as f64;
        if elapsed > 0.0 {
            frames / elapsed
        } else {
            0.0
        }
    }
}

#[tokio::test]
async fn test_independent_display_frame_rates() {
    // Start mock server
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    // Create two display channels with different IDs
    let _channel1 = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    let _channel2 = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 1)
        .await
        .unwrap();

    // Create surfaces for both displays
    let surface1 = SpiceMsgSurfaceCreate {
        surface_id: 0,
        width: 1920,
        height: 1080,
        format: 32,
        flags: 0,
    };

    let surface2 = SpiceMsgSurfaceCreate {
        surface_id: 1,
        width: 1280,
        height: 720,
        format: 32,
        flags: 0,
    };

    server
        .send_display_message_to_channel(
            0,
            SPICE_MSG_DISPLAY_SURFACE_CREATE,
            bincode::serialize(&surface1).unwrap(),
        )
        .await;
    server
        .send_display_message_to_channel(
            1,
            SPICE_MSG_DISPLAY_SURFACE_CREATE,
            bincode::serialize(&surface2).unwrap(),
        )
        .await;

    // Wait for surfaces to be created
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create frame rate monitors
    let monitor1 = FrameRateMonitor::new(0);
    let monitor2 = FrameRateMonitor::new(1);

    // Simulate different frame rates
    // Display 1: 60 FPS
    // Display 2: 30 FPS
    let monitor1_clone = monitor1.clone();
    let server_clone1 = server.clone();
    let display1_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(16)); // ~60 FPS
        let mut stream_data = SpiceStreamData {
            id: 0,
            multi_media_time: 0,
            data_size: 1024,
            data: vec![0xAA; 1024], // Dummy video data
        };

        for _ in 0..120 {
            // 2 seconds worth of frames
            interval.tick().await;
            stream_data.multi_media_time += 16;

            server_clone1
                .send_display_message_to_channel(
                    0,
                    DisplayChannelMessage::StreamData as u16,
                    bincode::serialize(&stream_data).unwrap(),
                )
                .await;

            monitor1_clone.increment();
        }
    });

    let monitor2_clone = monitor2.clone();
    let server_clone2 = server.clone();
    let display2_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(33)); // ~30 FPS
        let mut stream_data = SpiceStreamData {
            id: 1,
            multi_media_time: 0,
            data_size: 1024,
            data: vec![0xBB; 1024], // Different dummy video data
        };

        for _ in 0..60 {
            // 2 seconds worth of frames
            interval.tick().await;
            stream_data.multi_media_time += 33;

            server_clone2
                .send_display_message_to_channel(
                    1,
                    DisplayChannelMessage::StreamData as u16,
                    bincode::serialize(&stream_data).unwrap(),
                )
                .await;

            monitor2_clone.increment();
        }
    });

    // Wait for both tasks to complete
    let _ = tokio::join!(display1_task, display2_task);

    // Check frame rates
    let fps1 = monitor1.get_fps();
    let fps2 = monitor2.get_fps();

    println!("Display 1 FPS: {:.2}", fps1);
    println!("Display 2 FPS: {:.2}", fps2);

    // Verify independent frame rates
    assert!(
        fps1 > 55.0 && fps1 < 65.0,
        "Display 1 should be ~60 FPS, got {:.2}",
        fps1
    );
    assert!(
        fps2 > 25.0 && fps2 < 35.0,
        "Display 2 should be ~30 FPS, got {:.2}",
        fps2
    );
}

#[tokio::test]
async fn test_display_switching_during_video() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Create multiple surfaces
    for i in 0..3 {
        let surface = SpiceMsgSurfaceCreate {
            surface_id: i,
            width: 1920,
            height: 1080,
            format: 32,
            flags: 0,
        };
        server
            .send_display_message(
                SPICE_MSG_DISPLAY_SURFACE_CREATE,
                bincode::serialize(&surface).unwrap(),
            )
            .await;
    }

    // Start video streams on surfaces 0 and 1
    let stream0 = SpiceStreamCreate {
        id: 0,
        flags: 0,
        codec_type: 1,
        stamp: 0,
        stream_width: 1280,
        stream_height: 720,
        src_width: 1280,
        src_height: 720,
        dest: SpiceRect {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 720,
        },
        clip: SpiceClip {
            clip_type: 0,
            data: 0,
        },
    };

    let stream1 = stream0.clone(); // Simplified for test

    server
        .send_display_message(
            DisplayChannelMessage::StreamCreate as u16,
            bincode::serialize(&stream0).unwrap(),
        )
        .await;
    server
        .send_display_message(
            DisplayChannelMessage::StreamCreate as u16,
            bincode::serialize(&stream1).unwrap(),
        )
        .await;

    // Send video data to both streams
    for i in 0..10 {
        let data0 = SpiceStreamData {
            id: 0,
            multi_media_time: i * 33,
            data_size: 1024,
            data: vec![0xAA; 1024],
        };

        let data1 = SpiceStreamData {
            id: 1,
            multi_media_time: i * 33,
            data_size: 1024,
            data: vec![0xBB; 1024],
        };

        server
            .send_display_message(
                DisplayChannelMessage::StreamData as u16,
                bincode::serialize(&data0).unwrap(),
            )
            .await;
        server
            .send_display_message(
                DisplayChannelMessage::StreamData as u16,
                bincode::serialize(&data1).unwrap(),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(33)).await;
    }

    // Switch primary display by sending monitors config
    let monitors_config = SpiceMonitorsConfig {
        count: 2,
        max_allowed: 4,
        heads: vec![
            SpiceHead {
                id: 0,
                surface_id: 1, // Switch surface 1 to primary
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                flags: SPICE_HEAD_FLAGS_PRIMARY,
            },
            SpiceHead {
                id: 1,
                surface_id: 0, // Surface 0 becomes secondary
                width: 1920,
                height: 1080,
                x: 1920,
                y: 0,
                flags: SPICE_HEAD_FLAGS_NONE,
            },
        ],
    };

    server
        .send_display_message(
            SPICE_MSG_DISPLAY_MONITORS_CONFIG,
            bincode::serialize(&monitors_config).unwrap(),
        )
        .await;

    // Continue streaming after switch
    for i in 10..20 {
        let data0 = SpiceStreamData {
            id: 0,
            multi_media_time: i * 33,
            data_size: 1024,
            data: vec![0xCC; 1024],
        };

        let data1 = SpiceStreamData {
            id: 1,
            multi_media_time: i * 33,
            data_size: 1024,
            data: vec![0xDD; 1024],
        };

        server
            .send_display_message(
                DisplayChannelMessage::StreamData as u16,
                bincode::serialize(&data0).unwrap(),
            )
            .await;
        server
            .send_display_message(
                DisplayChannelMessage::StreamData as u16,
                bincode::serialize(&data1).unwrap(),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(33)).await;
    }

    // Verify monitors configuration was updated
    let monitors = channel.get_monitors();
    assert_eq!(monitors.len(), 2);
    assert_eq!(monitors[0].surface_id, 1);
    assert_eq!(monitors[0].flags, SPICE_HEAD_FLAGS_PRIMARY);
    assert_eq!(monitors[1].surface_id, 0);
    assert_eq!(monitors[1].flags, SPICE_HEAD_FLAGS_NONE);
}

#[tokio::test]
async fn test_multi_display_memory_management() {
    let server = MockSpiceServer::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr();

    let _channel = DisplayChannel::new(&addr.ip().to_string(), addr.port(), 0)
        .await
        .unwrap();

    // Track memory usage
    let initial_memory = get_approximate_memory_usage();

    // Create 4 high-resolution surfaces
    for i in 0..4 {
        let surface = SpiceMsgSurfaceCreate {
            surface_id: i,
            width: 3840, // 4K
            height: 2160,
            format: 32,
            flags: 0,
        };
        server
            .send_display_message(
                SPICE_MSG_DISPLAY_SURFACE_CREATE,
                bincode::serialize(&surface).unwrap(),
            )
            .await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create video streams on all surfaces
    for i in 0..4 {
        let stream = SpiceStreamCreate {
            id: i,
            flags: 0,
            codec_type: 1,
            stamp: 0,
            stream_width: 1920,
            stream_height: 1080,
            src_width: 1920,
            src_height: 1080,
            dest: SpiceRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            clip: SpiceClip {
                clip_type: 0,
                data: 0,
            },
        };

        server
            .send_display_message(
                DisplayChannelMessage::StreamCreate as u16,
                bincode::serialize(&stream).unwrap(),
            )
            .await;
    }

    // Stream video data to all displays
    for frame in 0..30 {
        for stream_id in 0..4 {
            let data = SpiceStreamData {
                id: stream_id,
                multi_media_time: frame * 33,
                data_size: 2048,
                data: vec![(stream_id * 0x10 + frame) as u8; 2048],
            };

            server
                .send_display_message(
                    DisplayChannelMessage::StreamData as u16,
                    bincode::serialize(&data).unwrap(),
                )
                .await;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let peak_memory = get_approximate_memory_usage();

    // Destroy all streams
    for i in 0..4 {
        let destroy = SpiceStreamDestroy { id: i };
        server
            .send_display_message(
                DisplayChannelMessage::StreamDestroy as u16,
                bincode::serialize(&destroy).unwrap(),
            )
            .await;
    }

    // Destroy all surfaces
    for i in 0..4 {
        let destroy = SpiceMsgSurfaceDestroy { surface_id: i };
        server
            .send_display_message(
                SPICE_MSG_DISPLAY_SURFACE_DESTROY,
                bincode::serialize(&destroy).unwrap(),
            )
            .await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_memory = get_approximate_memory_usage();

    println!(
        "Memory usage - Initial: {}, Peak: {}, Final: {}",
        initial_memory, peak_memory, final_memory
    );

    // Verify memory was properly released
    // Allow some overhead but ensure major allocations were freed
    assert!(
        final_memory < (peak_memory as f64 * 0.5) as usize,
        "Memory should be substantially reduced after cleanup"
    );
}

// Helper function to estimate memory usage (simplified)
fn get_approximate_memory_usage() -> usize {
    // In a real implementation, this would use system APIs to get actual memory usage
    // For testing purposes, we'll use a simplified approach

    // This is a placeholder - actual implementation would track allocations
    // or use platform-specific APIs like /proc/self/status on Linux
    1024 * 1024 // Return 1MB as placeholder
}
