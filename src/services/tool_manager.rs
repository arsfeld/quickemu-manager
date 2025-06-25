use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct ToolManager {
    tools_dir: PathBuf,
}

impl ToolManager {
    pub fn new() -> Self {
        let tools_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("quickemu-manager")
            .join("tools");
        
        Self { tools_dir }
    }

    pub async fn ensure_tools_available(&self) -> Result<(PathBuf, PathBuf)> {
        // Check if quickemu and quickget are already available in PATH
        if let (Ok(quickemu_path), Ok(quickget_path)) = (
            which::which("quickemu"),
            which::which("quickget")
        ) {
            println!("Found quickemu at: {}", quickemu_path.display());
            println!("Found quickget at: {}", quickget_path.display());
            return Ok((quickemu_path, quickget_path));
        }

        // Check if we have local copies
        let local_quickemu = self.tools_dir.join("quickemu");
        let local_quickget = self.tools_dir.join("quickget");
        
        if local_quickemu.exists() && local_quickget.exists() {
            println!("Using local tools at: {}", self.tools_dir.display());
            return Ok((local_quickemu, local_quickget));
        }

        // Download and install tools
        println!("Quickemu/quickget not found. Downloading...");
        self.download_tools().await?;
        
        Ok((local_quickemu, local_quickget))
    }

    async fn download_tools(&self) -> Result<()> {
        // Create tools directory
        fs::create_dir_all(&self.tools_dir).await?;
        
        let quickemu_url = "https://raw.githubusercontent.com/quickemu-project/quickemu/master/quickemu";
        let quickget_url = "https://raw.githubusercontent.com/quickemu-project/quickemu/master/quickget";
        
        // Download quickemu
        println!("Downloading quickemu...");
        self.download_script(quickemu_url, "quickemu").await?;
        
        // Download quickget
        println!("Downloading quickget...");
        self.download_script(quickget_url, "quickget").await?;
        
        println!("Tools downloaded successfully to: {}", self.tools_dir.display());
        Ok(())
    }

    async fn download_script(&self, url: &str, filename: &str) -> Result<()> {
        let response = reqwest::get(url).await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download {}: HTTP {}", filename, response.status()));
        }
        
        let content = response.text().await?;
        let file_path = self.tools_dir.join(filename);
        
        let mut file = fs::File::create(&file_path).await?;
        file.write_all(content.as_bytes()).await?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&file_path, perms).await?;
        }
        
        println!("Downloaded {} to {}", filename, file_path.display());
        Ok(())
    }

    pub fn get_tools_dir(&self) -> &Path {
        &self.tools_dir
    }

    pub async fn check_dependencies(&self) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        
        // Check for essential dependencies that quickemu needs
        let deps = ["qemu-system-x86_64", "wget", "unzip"];
        
        for dep in &deps {
            if which::which(dep).is_err() {
                missing.push(dep.to_string());
            }
        }
        
        Ok(missing)
    }

    pub async fn verify_tools(&self, quickemu_path: &Path, quickget_path: &Path) -> Result<()> {
        // Test quickemu
        let output = Command::new(quickemu_path)
            .arg("--version")
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("Quickemu verification failed"));
        }
        
        // Test quickget  
        let output = Command::new(quickget_path)
            .arg("--help")
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("Quickget verification failed"));
        }
        
        println!("Tools verified successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tool_manager_creation() {
        let tool_manager = ToolManager::new();
        assert!(tool_manager.get_tools_dir().ends_with("quickemu-manager/tools"));
    }

    #[tokio::test]
    async fn test_check_dependencies() {
        let tool_manager = ToolManager::new();
        let deps = tool_manager.check_dependencies().await.unwrap();
        // Should be able to check dependencies without error
        // (may or may not find missing deps depending on system)
    }
}