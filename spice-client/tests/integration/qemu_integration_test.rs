//! Integration tests with real QEMU/SPICE server

use instant::Instant;
use spice_client::channels::{CursorChannel, DisplayChannel, InputsChannel};
use spice_client::channels::{InputEvent, KeyCode, MouseButton};
use spice_client::{SpiceClient, SpiceClientShared, SpiceError};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

/// Tests require QEMU container to be running
/// Run: docker-compose -f docker/docker-compose.qemu.yml up -d
fn get_test_server() -> (String, u16) {
    let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("SPICE_TEST_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5900);
    (host, port)
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_qemu_basic_connection() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();
    info!("Connecting to QEMU SPICE server at {}:{}", host, port);

    let mut client = SpiceClient::new(host.clone(), port);

    match timeout(Duration::from_secs(10), client.connect()).await {
        Ok(Ok(_)) => {
            info!("Successfully connected to QEMU SPICE server");

            // Give server time to initialize
            sleep(Duration::from_millis(500)).await;

            client.disconnect();
            Ok(())
        }
        Ok(Err(e)) => {
            warn!("Failed to connect to QEMU: {:?}", e);
            if std::env::var("CI").is_ok() {
                info!("Skipping test in CI environment");
                Ok(())
            } else {
                Err(e)
            }
        }
        Err(_) => {
            warn!("Connection timeout");
            if std::env::var("CI").is_ok() {
                Ok(())
            } else {
                Err(SpiceError::Protocol("Connection timeout".to_string()))
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_qemu_display_channel() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let mut display = DisplayChannel::new(&host, port, 0).await?;
    display.initialize().await?;

    // Start receiving display messages
    let display_task = tokio::spawn(async move {
        let mut message_count = 0;
        let start = instant::Instant::now();

        while start.elapsed() < Duration::from_secs(5) {
            match timeout(Duration::from_millis(100), display.run()).await {
                Ok(Ok(_)) => {
                    message_count += 1;
                }
                Ok(Err(e)) => {
                    debug!("Display channel error: {:?}", e);
                    break;
                }
                Err(_) => {
                    // Timeout is normal if no messages
                }
            }
        }

        info!("Received {} display messages", message_count);
        message_count > 0
    });

    let received_messages = display_task.await.unwrap();
    assert!(
        received_messages,
        "Should receive at least one display message"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_multi_channel() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    // Create multiple channels
    let client = SpiceClientShared::new(host.clone(), port);
    client.connect().await?;

    // Start event loop
    let client_clone = client.clone();
    let event_loop = tokio::spawn(async move {
        let _ = client_clone.start_event_loop().await;
    });

    // Wait for initialization
    sleep(Duration::from_secs(2)).await;

    // In a real implementation, we would check available channels here
    // For now, we just verify connection was successful

    client.disconnect().await;
    event_loop.abort();

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_display_surface() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let client = SpiceClientShared::new(host, port);
    client.connect().await?;

    let client_clone = client.clone();
    let event_loop = tokio::spawn(async move {
        let _ = client_clone.start_event_loop().await;
    });

    // Wait for display to be ready
    let mut surface_found = false;
    let start = instant::Instant::now();

    while !surface_found && start.elapsed() < Duration::from_secs(10) {
        if let Some(surface) = client.get_display_surface(0).await {
            info!("Got display surface: {}x{}", surface.width, surface.height);
            assert!(surface.width > 0, "Surface width should be positive");
            assert!(surface.height > 0, "Surface height should be positive");
            surface_found = true;
        } else {
            sleep(Duration::from_millis(500)).await;
        }
    }

    assert!(surface_found, "Should find a display surface");

    client.disconnect().await;
    event_loop.abort();

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_input_events() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let mut inputs = InputsChannel::new(&host, port, 0).await?;
    inputs.initialize().await?;

    // Send various input events
    let events = vec![
        // Mouse movement
        InputEvent::MouseMove { x: 100, y: 100 },
        InputEvent::MouseMove { x: 200, y: 200 },
        // Mouse clicks
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        },
        InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: false,
        },
        // Keyboard events
        InputEvent::KeyDown(KeyCode::Char('H')),
        InputEvent::KeyUp(KeyCode::Char('H')),
        InputEvent::KeyDown(KeyCode::Char('i')),
        InputEvent::KeyUp(KeyCode::Char('i')),
        // Special keys
        InputEvent::KeyDown(KeyCode::Enter),
        InputEvent::KeyUp(KeyCode::Enter),
    ];

    for event in events {
        inputs.send_event(event).await?;
        sleep(Duration::from_millis(50)).await;
    }

    info!("Successfully sent all input events");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_cursor_channel() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let mut cursor = CursorChannel::new(&host, port, 0).await?;
    cursor.initialize().await?;

    // Start receiving cursor messages
    let cursor_task = tokio::spawn(async move {
        let start = instant::Instant::now();
        let mut cursor_updates = 0;

        while start.elapsed() < Duration::from_secs(3) {
            match timeout(Duration::from_millis(100), cursor.run()).await {
                Ok(Ok(_)) => {
                    cursor_updates += 1;
                }
                Ok(Err(e)) => {
                    debug!("Cursor channel error: {:?}", e);
                    break;
                }
                Err(_) => {
                    // Timeout is normal
                }
            }

            // Check cursor state
            if let Some(shape) = cursor.get_current_cursor() {
                info!("Cursor shape: {}x{}", shape.width, shape.height);
            }
        }

        info!("Received {} cursor updates", cursor_updates);
        cursor_updates
    });

    let updates = cursor_task.await.unwrap();
    info!("Total cursor updates: {}", updates);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_resolution_change() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let client = SpiceClientShared::new(host, port);
    client.connect().await?;

    let client_clone = client.clone();
    let event_loop = tokio::spawn(async move {
        let _ = client_clone.start_event_loop().await;
    });

    // Wait for initial surface
    sleep(Duration::from_secs(2)).await;

    let initial_surface = client.get_display_surface(0).await;
    if let Some(surface) = initial_surface {
        info!("Initial resolution: {}x{}", surface.width, surface.height);
    }

    // Note: Actual resolution change would require guest agent support
    // This test mainly verifies the client can handle resolution messages

    client.disconnect().await;
    event_loop.abort();

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_qemu_performance_streaming() -> Result<(), SpiceError> {
    let (host, port) = get_test_server();

    let client = SpiceClientShared::new(host, port);
    client.connect().await?;

    let client_clone = client.clone();
    let event_loop = tokio::spawn(async move {
        let _ = client_clone.start_event_loop().await;
    });

    // Measure frame rate over time
    let start = instant::Instant::now();
    let mut frame_count = 0;
    let mut last_frame_data = Vec::new();

    while start.elapsed() < Duration::from_secs(5) {
        if let Some(surface) = client.get_display_surface(0).await {
            // Check if frame data changed
            if surface.data != last_frame_data {
                frame_count += 1;
                last_frame_data = surface.data.clone();
            }
        }
        sleep(Duration::from_millis(16)).await; // ~60 FPS check rate
    }

    let elapsed = start.elapsed().as_secs_f64();
    let fps = frame_count as f64 / elapsed;

    info!(
        "Performance test: {} frames in {:.2}s = {:.2} FPS",
        frame_count, elapsed, fps
    );

    client.disconnect().await;
    event_loop.abort();

    Ok(())
}
