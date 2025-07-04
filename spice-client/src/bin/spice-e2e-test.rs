use clap::Parser;
use spice_client::SpiceClientShared;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about = "SPICE client end-to-end test program", long_about = None)]
struct Args {
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    #[arg(short, long, default_value = "5900")]
    port: u16,

    #[arg(short = 'd', long, default_value = "30")]
    duration: u64,

    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(short = 'P', long)]
    password: Option<String>,

    #[arg(long, help = "Fail if no display updates are received")]
    require_display_updates: bool,

    #[arg(long, help = "Fail if display channel is not connected")]
    require_display_channel: bool,

    #[arg(long, help = "Test mouse input events")]
    test_mouse_input: bool,

    #[arg(long, help = "Test keyboard input events")]
    test_keyboard_input: bool,
}

struct TestMetrics {
    display_updates_received: AtomicU32,
    display_channel_connected: AtomicBool,
    inputs_channel_connected: AtomicBool,
    cursor_channel_connected: AtomicBool,
    last_display_width: AtomicU32,
    last_display_height: AtomicU32,
    errors_encountered: AtomicU32,
}

impl TestMetrics {
    fn new() -> Self {
        Self {
            display_updates_received: AtomicU32::new(0),
            display_channel_connected: AtomicBool::new(false),
            inputs_channel_connected: AtomicBool::new(false),
            cursor_channel_connected: AtomicBool::new(false),
            last_display_width: AtomicU32::new(0),
            last_display_height: AtomicU32::new(0),
            errors_encountered: AtomicU32::new(0),
        }
    }

    fn print_summary(&self) {
        info!("=== E2E Test Summary ===");
        info!(
            "Display updates received: {}",
            self.display_updates_received.load(Ordering::Relaxed)
        );
        info!(
            "Display channel connected: {}",
            self.display_channel_connected.load(Ordering::Relaxed)
        );
        info!(
            "Inputs channel connected: {}",
            self.inputs_channel_connected.load(Ordering::Relaxed)
        );
        info!(
            "Cursor channel connected: {}",
            self.cursor_channel_connected.load(Ordering::Relaxed)
        );
        info!(
            "Last display size: {}x{}",
            self.last_display_width.load(Ordering::Relaxed),
            self.last_display_height.load(Ordering::Relaxed)
        );
        info!(
            "Errors encountered: {}",
            self.errors_encountered.load(Ordering::Relaxed)
        );
        info!("=======================");
    }
}

async fn test_mouse_movements(
    client: &SpiceClientShared,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing mouse input events...");

    // Test pattern: move mouse in a square pattern
    let positions = vec![(100, 100), (500, 100), (500, 500), (100, 500), (100, 100)];

    for (x, y) in positions {
        debug!("Moving mouse to ({}, {})", x, y);
        client.send_mouse_motion(0, x, y).await?;
        sleep(Duration::from_millis(100)).await;
    }

    // Test mouse clicks
    use spice_client::channels::MouseButton;
    for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
        debug!("Testing {:?} mouse button", button);
        client.send_mouse_button(0, button, true).await?;
        sleep(Duration::from_millis(50)).await;
        client.send_mouse_button(0, button, false).await?;
        sleep(Duration::from_millis(50)).await;
    }

    // Test mouse wheel
    debug!("Testing mouse wheel");
    client.send_mouse_wheel(0, 0, 5).await?;
    sleep(Duration::from_millis(50)).await;
    client.send_mouse_wheel(0, 0, -5).await?;

    info!("Mouse input tests completed");
    Ok(())
}

async fn test_keyboard_input(client: &SpiceClientShared) -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing keyboard input events...");

    // Test some common scancodes (A-Z keys)
    let test_keys = vec![
        (0x1E, "A"),      // A key
        (0x30, "B"),      // B key
        (0x2E, "C"),      // C key
        (0x20, "D"),      // D key
        (0x12, "E"),      // E key
        (0x01, "Escape"), // Escape key
        (0x1C, "Enter"),  // Enter key
        (0x39, "Space"),  // Space key
    ];

    for (scancode, key_name) in test_keys {
        debug!("Testing key: {} (scancode: 0x{:02X})", key_name, scancode);
        client.send_key_down(0, scancode).await?;
        sleep(Duration::from_millis(50)).await;
        client.send_key_up(0, scancode).await?;
        sleep(Duration::from_millis(50)).await;
    }

    // Test key combination (Ctrl+A)
    debug!("Testing key combination: Ctrl+A");
    client.send_key_down(0, 0x1D).await?; // Ctrl down
    sleep(Duration::from_millis(50)).await;
    client.send_key_down(0, 0x1E).await?; // A down
    sleep(Duration::from_millis(50)).await;
    client.send_key_up(0, 0x1E).await?; // A up
    sleep(Duration::from_millis(50)).await;
    client.send_key_up(0, 0x1D).await?; // Ctrl up

    info!("Keyboard input tests completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup logging
    let log_level = match args.verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting SPICE E2E test client");
    info!("Test configuration:");
    info!("  Host: {}:{}", args.host, args.port);
    info!("  Duration: {} seconds", args.duration);
    info!(
        "  Require display updates: {}",
        args.require_display_updates
    );
    info!(
        "  Require display channel: {}",
        args.require_display_channel
    );
    info!("  Test mouse input: {}", args.test_mouse_input);
    info!("  Test keyboard input: {}", args.test_keyboard_input);

    let metrics = Arc::new(TestMetrics::new());
    let mut client = SpiceClientShared::new(args.host.clone(), args.port);

    if let Some(password) = args.password {
        info!("Setting password");
        client.set_password(password).await;
    }

    // Connect to SPICE server
    match client.connect().await {
        Ok(()) => {
            info!("✓ Successfully connected to SPICE server");

            // Check which channels are connected
            let start_check = Instant::now();
            while start_check.elapsed() < Duration::from_secs(2) {
                if client.get_display_surface(0).await.is_some() {
                    metrics
                        .display_channel_connected
                        .store(true, Ordering::Relaxed);
                    info!("✓ Display channel 0 is connected");
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }

            // Set up display update callback
            let metrics_clone = metrics.clone();
            match client
                .set_display_update_callback(0, move |surface| {
                    let count = metrics_clone
                        .display_updates_received
                        .fetch_add(1, Ordering::Relaxed)
                        + 1;
                    metrics_clone
                        .last_display_width
                        .store(surface.width, Ordering::Relaxed);
                    metrics_clone
                        .last_display_height
                        .store(surface.height, Ordering::Relaxed);
                    debug!(
                        "Display update #{}: {}x{}, format: {:?}",
                        count, surface.width, surface.height, surface.format
                    );
                })
                .await
            {
                Ok(()) => info!("✓ Display update callback registered"),
                Err(e) => warn!("Could not set display callback: {}", e),
            }

            // Start event loops
            match client.start_event_loop().await {
                Ok(()) => {
                    info!("✓ Event loops started successfully");

                    // Run input tests if requested
                    if args.test_mouse_input {
                        if let Err(e) = test_mouse_movements(&client).await {
                            error!("Mouse input tests failed: {}", e);
                            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    if args.test_keyboard_input {
                        if let Err(e) = test_keyboard_input(&client).await {
                            error!("Keyboard input tests failed: {}", e);
                            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    // Monitor for the specified duration
                    info!("Running for {} seconds...", args.duration);
                    let start = Instant::now();
                    let mut last_report = Instant::now();

                    while start.elapsed().as_secs() < args.duration {
                        // Check display surface periodically
                        if let Some(surface) = client.get_display_surface(0).await {
                            if last_report.elapsed() > Duration::from_secs(5) {
                                info!(
                                    "Display surface: {}x{}, format: {:?}",
                                    surface.width, surface.height, surface.format
                                );
                                info!(
                                    "Display updates received so far: {}",
                                    metrics.display_updates_received.load(Ordering::Relaxed)
                                );
                                last_report = Instant::now();
                            }
                        }

                        // Check cursor shape
                        if let Some(cursor) = client.get_cursor_shape(0).await {
                            if !metrics.cursor_channel_connected.load(Ordering::Relaxed) {
                                metrics
                                    .cursor_channel_connected
                                    .store(true, Ordering::Relaxed);
                                info!("✓ Cursor channel 0 is connected");
                                debug!(
                                    "Cursor shape: {}x{}, hotspot: ({}, {})",
                                    cursor.width,
                                    cursor.height,
                                    cursor.hot_spot_x,
                                    cursor.hot_spot_y
                                );
                            }
                        }

                        sleep(Duration::from_millis(100)).await;
                    }

                    info!("Test duration complete, disconnecting...");
                    client.disconnect().await;
                    info!("✓ Disconnected successfully");

                    // Print final metrics
                    metrics.print_summary();

                    // Check test requirements
                    let mut test_passed = true;

                    if args.require_display_channel
                        && !metrics.display_channel_connected.load(Ordering::Relaxed)
                    {
                        error!("FAIL: Display channel was required but not connected");
                        test_passed = false;
                    }

                    if args.require_display_updates
                        && metrics.display_updates_received.load(Ordering::Relaxed) == 0
                    {
                        error!("FAIL: Display updates were required but none received");
                        test_passed = false;
                    }

                    if metrics.errors_encountered.load(Ordering::Relaxed) > 0 {
                        error!(
                            "FAIL: {} errors encountered during test",
                            metrics.errors_encountered.load(Ordering::Relaxed)
                        );
                        test_passed = false;
                    }

                    if test_passed {
                        info!("✓ All E2E tests PASSED");
                        Ok(())
                    } else {
                        Err("E2E tests FAILED".into())
                    }
                }
                Err(e) => {
                    error!("Failed to start event loop: {}", e);
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to SPICE server: {}", e);
            error!(
                "Make sure QEMU is running with SPICE enabled on {}:{}",
                args.host, args.port
            );

            // Check for specific error types
            if e.to_string().contains("BAD_CONNECTION_ID") {
                error!("Protocol error: BAD_CONNECTION_ID - Check connection_id handling");
            } else if e.to_string().contains("refused") {
                error!("Connection refused - Is the SPICE server running?");
            } else if e.to_string().contains("timeout") {
                error!("Connection timeout - Check network connectivity");
            }

            Err(e.into())
        }
    }
}
