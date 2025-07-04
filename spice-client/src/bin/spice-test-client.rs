use clap::Parser;
use spice_client::SpiceClientShared;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about = "SPICE client test program", long_about = None)]
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup logging
    let log_level = match args.verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting SPICE test client");
    info!("Connecting to {}:{}", args.host, args.port);

    let mut client = SpiceClientShared::new(args.host.clone(), args.port);

    if let Some(password) = args.password {
        info!("Setting password");
        client.set_password(password).await;
    }

    match client.connect().await {
        Ok(()) => {
            info!("Successfully connected to SPICE server");

            match client.start_event_loop().await {
                Ok(()) => {
                    info!("Event loop started successfully");
                    info!("Running for {} seconds...", args.duration);

                    // Run for specified duration, checking display updates
                    let start = std::time::Instant::now();
                    while start.elapsed().as_secs() < args.duration {
                        // Try to get display surface
                        if let Some(surface) = client.get_display_surface(0).await {
                            info!(
                                "Display surface available: {}x{}, format: {:?}",
                                surface.width, surface.height, surface.format
                            );
                        }

                        sleep(Duration::from_secs(1)).await;
                    }

                    info!("Test duration complete, disconnecting...");
                    client.disconnect().await;
                    info!("Disconnected successfully");
                }
                Err(e) => {
                    error!("Failed to start event loop: {}", e);
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to SPICE server: {}", e);
            error!(
                "Make sure QEMU is running with SPICE enabled on {}:{}",
                args.host, args.port
            );

            // Check if it's an authentication error with display channel
            if e.to_string().contains("BAD_CONNECTION_ID") {
                error!("CRITICAL: BAD_CONNECTION_ID indicates incomplete SPICE protocol implementation");
                error!(
                    "The client is not properly handling connection IDs or channel initialization"
                );
                error!("This needs to be debugged - check protocol.rs and channels/mod.rs");
            }

            return Err(e.into());
        }
    }

    info!("Test client finished");
    Ok(())
}
