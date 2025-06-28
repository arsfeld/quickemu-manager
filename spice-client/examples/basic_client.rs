//! Basic example of using the spice-client library

use spice_client::{SpiceClient, SpiceError};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), SpiceError> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a new SPICE client
    let host = std::env::var("SPICE_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("SPICE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5900);

    println!("Connecting to SPICE server at {}:{}", host, port);
    
    let mut client = SpiceClient::new(host, port);
    
    // Connect to the SPICE server
    match client.connect().await {
        Ok(_) => println!("Connected successfully!"),
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
            return Err(e);
        }
    }
    
    // Start the event loop in the background
    let client_handle = tokio::spawn(async move {
        if let Err(e) = client.start_event_loop().await {
            eprintln!("Event loop error: {:?}", e);
        }
    });
    
    // Let the client run for a while
    println!("Client is running. Press Ctrl+C to stop.");
    
    // In a real application, you would:
    // 1. Get display surfaces and render them
    // 2. Send input events (keyboard/mouse)
    // 3. Handle audio/video streams
    // 4. etc.
    
    // For this example, just wait
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down...");
        }
        _ = client_handle => {
            println!("Client disconnected");
        }
    }
    
    Ok(())
}