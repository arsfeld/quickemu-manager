use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Service for discovering quickemu and quickget binaries
#[derive(Debug, Clone)]
pub struct BinaryDiscovery {
    quickemu_path: Option<PathBuf>,
    quickget_path: Option<PathBuf>,
}

impl BinaryDiscovery {
    /// Create a new binary discovery service and perform initial discovery
    pub async fn new() -> Self {
        let mut service = Self {
            quickemu_path: None,
            quickget_path: None,
        };

        service.discover_binaries().await;
        service
    }

    /// Create a new binary discovery service without performing discovery
    pub fn new_without_discovery() -> Self {
        Self {
            quickemu_path: None,
            quickget_path: None,
        }
    }

    /// Discover quickemu and quickget binaries using multiple strategies
    pub async fn discover_binaries(&mut self) {
        self.quickemu_path = Self::find_binary("quickemu").await;
        self.quickget_path = Self::find_binary("quickget").await;
    }

    /// Find a binary using multiple discovery strategies
    async fn find_binary(binary_name: &str) -> Option<PathBuf> {
        // Strategy 1: Use which crate (searches PATH)
        if let Ok(path) = which::which(binary_name) {
            return Some(path);
        }

        // Strategy 2: Try system which command as fallback
        if let Some(path) = Self::find_binary_with_which_command(binary_name) {
            return Some(path);
        }

        // Strategy 3: Check our local quickemu installation
        if let Some(path) = Self::find_binary_in_local_quickemu(binary_name) {
            return Some(path);
        }

        // Strategy 4: Download quickemu if not found anywhere
        if let Ok(path) = Self::download_and_install_quickemu(binary_name).await {
            return Some(path);
        }

        None
    }

    /// Use system 'which' command to find binary
    fn find_binary_with_which_command(binary_name: &str) -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg(binary_name).output() {
            if output.status.success() {
                let path_string = String::from_utf8_lossy(&output.stdout);
                let path_str = path_string.trim();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        None
    }

    /// Check our local quickemu installation directory
    fn find_binary_in_local_quickemu(binary_name: &str) -> Option<PathBuf> {
        if let Some(quickemu_dir) = Self::get_quickemu_dir() {
            let binary_path = quickemu_dir.join(binary_name);
            if binary_path.exists() && Self::is_executable(&binary_path) {
                return Some(binary_path);
            }
        }
        None
    }

    /// Get the directory where we store our local quickemu installation
    fn get_quickemu_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|data_dir| data_dir.join("quickemu-manager").join("quickemu"))
    }

    /// Download and install quickemu from GitHub
    async fn download_and_install_quickemu(binary_name: &str) -> Result<PathBuf> {
        let quickemu_dir = Self::get_quickemu_dir()
            .ok_or_else(|| anyhow!("Could not determine local data directory"))?;

        // Create the directory if it doesn't exist
        fs::create_dir_all(&quickemu_dir)?;

        // Check if we already have the binary after a previous download
        let binary_path = quickemu_dir.join(binary_name);
        if binary_path.exists() && Self::is_executable(&binary_path) {
            return Ok(binary_path);
        }

        println!("Downloading quickemu from GitHub...");

        // Download the quickemu release
        let download_url =
            "https://github.com/quickemu-project/quickemu/archive/refs/tags/4.9.7.zip";
        let response = reqwest::get(download_url).await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download quickemu: HTTP {}",
                response.status()
            ));
        }

        let zip_data = response.bytes().await?;

        // Extract the zip file
        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)?;

        // Extract specific files we need
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.name();

            // Look for quickemu and quickget in the extracted archive
            if file_path.ends_with("/quickemu") || file_path.ends_with("/quickget") {
                let path_buf = PathBuf::from(file_path);
                let filename = path_buf
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");

                if filename == "quickemu" || filename == "quickget" {
                    let dest_path = quickemu_dir.join(filename);
                    let mut dest_file = fs::File::create(&dest_path)?;
                    std::io::copy(&mut file, &mut dest_file)?;

                    // Make executable
                    Self::make_executable(&dest_path)?;

                    println!("Extracted {filename}");
                }
            }
        }

        // Return the requested binary path
        let binary_path = quickemu_dir.join(binary_name);
        if binary_path.exists() && Self::is_executable(&binary_path) {
            println!(
                "✅ Successfully installed {} to {}",
                binary_name,
                binary_path.display()
            );
            Ok(binary_path)
        } else {
            Err(anyhow!(
                "Failed to extract {} from quickemu archive",
                binary_name
            ))
        }
    }

    /// Make a file executable
    fn make_executable(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        fs::set_permissions(path, perms)?;
        Ok(())
    }

    /// Check if a file is executable
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = std::fs::metadata(path) {
            let permissions = metadata.permissions();
            // Check if any execute bit is set (owner, group, or other)
            permissions.mode() & 0o111 != 0
        } else {
            false
        }
    }

    /// Get the discovered quickemu binary path
    pub fn quickemu_path(&self) -> Result<&Path> {
        self.quickemu_path
            .as_deref()
            .ok_or_else(|| {
                anyhow!("quickemu not found. Please install quickemu or ensure it's in your PATH.")
            })
    }

    /// Get the discovered quickget binary path (optional)
    pub fn quickget_path(&self) -> Option<&Path> {
        self.quickget_path.as_deref()
    }

    /// Check if quickemu is available
    pub fn has_quickemu(&self) -> bool {
        self.quickemu_path.is_some()
    }

    /// Check if quickget is available
    pub fn has_quickget(&self) -> bool {
        self.quickget_path.is_some()
    }

    /// Get both paths as PathBuf for use in VMManager
    pub fn get_paths(&self) -> (Option<PathBuf>, Option<PathBuf>) {
        (self.quickemu_path.clone(), self.quickget_path.clone())
    }

    /// Force refresh of binary discovery
    pub async fn refresh(&mut self) {
        self.discover_binaries().await;
    }

    /// Create a BinaryDiscovery with specific paths (for testing or custom configs)
    pub fn with_paths(quickemu_path: Option<PathBuf>, quickget_path: Option<PathBuf>) -> Self {
        Self {
            quickemu_path,
            quickget_path,
        }
    }

    /// Validate that the discovered binaries are actually executable
    pub fn validate(&self) -> Result<()> {
        // Quickemu is required
        let quickemu_path = self.quickemu_path()?;
        if !Self::is_executable(quickemu_path) {
            return Err(anyhow!(
                "quickemu binary at {} is not executable",
                quickemu_path.display()
            ));
        }

        // Quickget is optional but should be executable if present
        if let Some(quickget_path) = &self.quickget_path {
            if !Self::is_executable(quickget_path) {
                return Err(anyhow!(
                    "quickget binary at {} is not executable",
                    quickget_path.display()
                ));
            }
        }

        Ok(())
    }

    /// Get detailed discovery information for debugging
    pub fn discovery_info(&self) -> String {
        let mut info = String::new();

        match &self.quickemu_path {
            Some(path) => info.push_str(&format!("quickemu: {} ✓\n", path.display())),
            None => info.push_str("quickemu: NOT FOUND ✗ (will attempt download)\n"),
        }

        match &self.quickget_path {
            Some(path) => info.push_str(&format!("quickget: {} ✓\n", path.display())),
            None => info.push_str("quickget: NOT FOUND ✗ (will attempt download)\n"),
        }

        if let Some(quickemu_dir) = Self::get_quickemu_dir() {
            info.push_str(&format!(
                "Local quickemu directory: {}\n",
                quickemu_dir.display()
            ));
        }

        info
    }
}

impl Default for BinaryDiscovery {
    fn default() -> Self {
        Self::new_without_discovery()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_binary_discovery_creation() {
        let discovery = BinaryDiscovery::new().await;
        // Should not panic and should attempt discovery
        let _info = discovery.discovery_info();
    }

    #[test]
    fn test_binary_discovery_with_paths() {
        let quickemu_path = Some(PathBuf::from("/usr/bin/echo"));
        let quickget_path = Some(PathBuf::from("/usr/bin/true"));

        let discovery = BinaryDiscovery::with_paths(quickemu_path.clone(), quickget_path.clone());

        assert_eq!(discovery.quickemu_path, quickemu_path);
        assert_eq!(discovery.quickget_path, quickget_path);
    }

    #[test]
    fn test_has_binaries() {
        let discovery = BinaryDiscovery::with_paths(Some(PathBuf::from("/usr/bin/echo")), None);

        assert!(discovery.has_quickemu());
        assert!(!discovery.has_quickget());
    }

    #[test]
    fn test_get_paths() {
        let quickemu_path = Some(PathBuf::from("/usr/bin/echo"));
        let quickget_path = Some(PathBuf::from("/usr/bin/true"));

        let discovery = BinaryDiscovery::with_paths(quickemu_path.clone(), quickget_path.clone());
        let (qemu_path, qget_path) = discovery.get_paths();

        assert_eq!(qemu_path, quickemu_path);
        assert_eq!(qget_path, quickget_path);
    }

    #[test]
    fn test_is_executable() {
        // Test with a known executable
        let executable_path = PathBuf::from("/usr/bin/echo");
        if executable_path.exists() {
            assert!(BinaryDiscovery::is_executable(&executable_path));
        }
    }

    #[test]
    fn test_discovery_info() {
        let discovery = BinaryDiscovery::with_paths(Some(PathBuf::from("/usr/bin/echo")), None);

        let info = discovery.discovery_info();
        assert!(info.contains("quickemu"));
        assert!(info.contains("quickget"));
        assert!(info.contains("✓"));
        assert!(info.contains("✗"));
    }

    #[test]
    fn test_find_binary_in_local_quickemu() {
        // This test verifies the function works
        let result = BinaryDiscovery::find_binary_in_local_quickemu("definitely_not_a_real_binary");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_quickemu_dir() {
        let dir = BinaryDiscovery::get_quickemu_dir();
        // Should return Some path on most systems
        if let Some(path) = dir {
            assert!(path.to_string_lossy().contains("quickemu-manager"));
            assert!(path.to_string_lossy().contains("quickemu"));
        }
    }
}
