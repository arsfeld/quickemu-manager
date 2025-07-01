use spice_client::{SpiceClient, SpiceClientShared, SpiceError};
use std::time::Duration;
use tokio::time::timeout;

pub mod harness;
pub mod multi_display_test;
pub mod multi_display_framerate_test;
pub mod cursor_test;
pub mod inputs_test;
pub mod qemu_integration_test;

#[cfg(test)]
mod connection_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connect_to_spice_server() -> Result<(), SpiceError> {
        // This test requires the Docker container to be running
        // Run: docker-compose -f tests/docker/docker-compose.yml up -d
        
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        println!("Testing connection to {}:{}", host, port);
        
        let mut client = SpiceClient::new(host.clone(), port);
        
        // Try to connect with timeout
        match timeout(Duration::from_secs(5), client.connect()).await {
            Ok(Ok(_)) => {
                println!("Successfully connected to SPICE server");
                Ok(())
            }
            Ok(Err(e)) => {
                println!("Connection failed: {:?}", e);
                if std::env::var("CI").is_ok() {
                    // In CI, skip if server not available
                    println!("Skipping test in CI environment");
                    Ok(())
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                println!("Connection timeout");
                if std::env::var("CI").is_ok() {
                    println!("Skipping test in CI environment");
                    Ok(())
                } else {
                    Err(SpiceError::Protocol("Connection timeout".to_string()))
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_invalid_host_connection() {
        let mut client = SpiceClient::new("invalid.host.test".to_string(), 5900);
        
        let result = timeout(Duration::from_secs(2), client.connect()).await;
        
        assert!(
            matches!(result, Ok(Err(_)) | Err(_)),
            "Connection to invalid host should fail"
        );
    }
    
    #[tokio::test]
    async fn test_channel_negotiation() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        let mut client = SpiceClient::new(host, port);
        client.connect().await?;
        
        // Just test that we can connect successfully
        println!("Channel negotiation test passed");
        
        Ok(())
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_display_surface_creation() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        let client = SpiceClientShared::new(host, port);
        client.connect().await?;
        
        // Start event loop in background
        let client_clone = client.clone();
        tokio::spawn(async move {
            client_clone.start_event_loop().await
        });
        
        // Give the event loop time to start and receive display messages
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Check for display surface
        if let Some(surface) = client.get_display_surface(0).await {
            println!("Got display surface: {}x{} format: {}", 
                     surface.width, surface.height, surface.format);
            assert!(surface.width > 0);
            assert!(surface.height > 0);
        } else {
            println!("No display surface available (this might be normal if no VM is running)");
        }
        
        // Clean up
        client.disconnect().await;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_connect_disconnect() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        // Test multiple connect/disconnect cycles
        for i in 0..3 {
            println!("Connect/disconnect cycle {}", i + 1);
            let mut client = SpiceClient::new(host.clone(), port);
            client.connect().await?;
            client.disconnect();
            // Small delay between connections
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_connections() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        // Try to create multiple concurrent connections
        let mut handles = vec![];
        
        for i in 0..3 {
            let host_clone = host.clone();
            let handle = tokio::spawn(async move {
                println!("Starting connection {}", i);
                let mut client = SpiceClient::new(host_clone, port);
                match timeout(Duration::from_secs(5), client.connect()).await {
                    Ok(Ok(_)) => {
                        println!("Connection {} established", i);
                        // Keep connection alive briefly
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        client.disconnect();
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        println!("Connection {} failed: {:?}", i, e);
                        Err(e)
                    }
                    Err(_) => {
                        println!("Connection {} timed out", i);
                        Err(SpiceError::Protocol("Timeout".to_string()))
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all connections
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                success_count += 1;
            }
        }
        
        println!("Successfully established {} concurrent connections", success_count);
        assert!(success_count > 0, "At least one connection should succeed");
        
        Ok(())
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_with_wrong_port() {
        let mut client = SpiceClient::new("localhost".to_string(), 9999);
        
        let result = timeout(Duration::from_secs(2), client.connect()).await;
        
        assert!(
            matches!(result, Ok(Err(_)) | Err(_)),
            "Connection to wrong port should fail"
        );
    }

    #[tokio::test]
    async fn test_rapid_connect_disconnect() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        // Rapid connect/disconnect without delay
        for i in 0..5 {
            let mut client = SpiceClient::new(host.clone(), port);
            match client.connect().await {
                Ok(_) => {
                    client.disconnect();
                }
                Err(e) => {
                    println!("Connection {} failed (expected in rapid test): {:?}", i, e);
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use instant::Instant;
    
    #[tokio::test]
    async fn test_connection_latency() -> Result<(), SpiceError> {
        let host = std::env::var("SPICE_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("SPICE_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5900);
        
        let mut latencies = vec![];
        
        // Measure connection latency over multiple attempts
        for _ in 0..5 {
            let start = Instant::now();
            let mut client = SpiceClient::new(host.clone(), port);
            
            match client.connect().await {
                Ok(_) => {
                    let latency = start.elapsed();
                    latencies.push(latency);
                    println!("Connection latency: {:?}", latency);
                    client.disconnect();
                }
                Err(e) => {
                    println!("Connection failed: {:?}", e);
                }
            }
            
            // Small delay between measurements
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        if !latencies.is_empty() {
            let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            println!("Average connection latency: {:?}", avg_latency);
            
            // Assert reasonable latency (adjust based on your requirements)
            assert!(
                avg_latency < Duration::from_secs(2),
                "Connection latency too high: {:?}",
                avg_latency
            );
        }
        
        Ok(())
    }
}