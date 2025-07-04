use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Test harness for integration tests
pub struct TestHarness {
    container_name: String,
}

impl TestHarness {
    pub fn new() -> Self {
        Self {
            container_name: "spice-test-server".to_string(),
        }
    }

    /// Start the Docker container with SPICE server
    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting SPICE test server...");

        // Check if Docker is available
        let docker_check = Command::new("docker").arg("version").output();

        if docker_check.is_err() {
            return Err("Docker not available".into());
        }

        // Build the image
        let build_result = Command::new("docker-compose")
            .arg("-f")
            .arg("tests/docker/docker-compose.yml")
            .arg("build")
            .output()?;

        if !build_result.status.success() {
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            return Err(format!("Failed to build Docker image: {}", stderr).into());
        }

        // Start the container
        let start_result = Command::new("docker-compose")
            .arg("-f")
            .arg("tests/docker/docker-compose.yml")
            .arg("up")
            .arg("-d")
            .output()?;

        if !start_result.status.success() {
            let stderr = String::from_utf8_lossy(&start_result.stderr);
            return Err(format!("Failed to start container: {}", stderr).into());
        }

        // Wait for container to be ready
        println!("Waiting for SPICE server to be ready...");
        for i in 0..30 {
            if self.is_server_ready().await {
                println!("SPICE server is ready!");
                return Ok(());
            }
            println!("Waiting... ({}/30)", i + 1);
            sleep(Duration::from_secs(1)).await;
        }

        Err("SPICE server failed to start within timeout".into())
    }

    /// Check if the SPICE server is ready
    pub async fn is_server_ready(&self) -> bool {
        let output = Command::new("docker")
            .args(&[
                "exec",
                &self.container_name,
                "nc",
                "-z",
                "localhost",
                "5900",
            ])
            .output();

        matches!(output, Ok(result) if result.status.success())
    }

    /// Stop the Docker container
    pub async fn stop_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Stopping SPICE test server...");

        let stop_result = Command::new("docker-compose")
            .arg("-f")
            .arg("tests/docker/docker-compose.yml")
            .arg("down")
            .output()?;

        if !stop_result.status.success() {
            let stderr = String::from_utf8_lossy(&stop_result.stderr);
            return Err(format!("Failed to stop container: {}", stderr).into());
        }

        Ok(())
    }

    /// Get container logs
    pub fn get_logs(&self) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("docker")
            .args(&["logs", &self.container_name])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Try to stop the server when test harness is dropped
        let _ = std::process::Command::new("docker-compose")
            .arg("-f")
            .arg("tests/docker/docker-compose.yml")
            .arg("down")
            .output();
    }
}

/// Run a test with SPICE server
pub async fn with_spice_server<F, Fut>(test_fn: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let harness = TestHarness::new();

    // Start server
    harness.start_server().await?;

    // Run test
    let result = test_fn().await;

    // Get logs if test failed
    if result.is_err() {
        if let Ok(logs) = harness.get_logs() {
            println!("=== SPICE Server Logs ===");
            println!("{}", logs);
            println!("========================");
        }
    }

    // Stop server
    harness.stop_server().await?;

    result
}
