use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub vm_directories: Vec<PathBuf>,
    pub quickemu_path: Option<PathBuf>,
    pub quickget_path: Option<PathBuf>,
    pub auto_download_tools: bool,
    pub theme: Theme,
    pub update_interval_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut vm_dirs = vec![];

        if let Some(home) = dirs::home_dir() {
            vm_dirs.push(home.join(".config/quickemu"));
            vm_dirs.push(home.join("VMs"));
        }

        Self {
            vm_directories: vm_dirs,
            quickemu_path: None,
            quickget_path: None,
            auto_download_tools: true,
            theme: Theme::System,
            update_interval_ms: 1000,
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let config_path = config_dir.join("quickemu-manager").join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let app_config_dir = config_dir.join("quickemu-manager");

        std::fs::create_dir_all(&app_config_dir)?;

        let config_path = app_config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;

        Ok(())
    }

    /// Get the primary VM directory (first directory, used for creating new VMs)
    pub fn get_primary_vm_directory(&self) -> PathBuf {
        self.vm_directories.first().cloned().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|home| home.join("VMs"))
                .unwrap_or_else(|| PathBuf::from("/tmp/VMs"))
        })
    }

    /// Get all VM directories for discovery
    pub fn get_all_vm_directories(&self) -> &Vec<PathBuf> {
        &self.vm_directories
    }

    /// Set the primary VM directory (moves it to first position)
    pub fn set_primary_vm_directory(&mut self, directory: PathBuf) {
        // Remove the directory if it already exists
        self.vm_directories.retain(|d| d != &directory);
        // Insert at the beginning (primary position)
        self.vm_directories.insert(0, directory);
    }

    /// Add a VM directory for discovery
    pub fn add_vm_directory(&mut self, directory: PathBuf) {
        if !self.vm_directories.contains(&directory) {
            self.vm_directories.push(directory);
        }
    }

    /// Remove a VM directory
    pub fn remove_vm_directory(&mut self, directory: &PathBuf) {
        self.vm_directories.retain(|d| d != directory);
    }
}
