use crate::models::config::AppConfig;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Centralized configuration manager for the application
#[derive(Clone)]
pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigManager {
    /// Create a new ConfigManager and load configuration
    pub async fn new() -> Result<Self> {
        let config = AppConfig::load()?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
        })
    }

    /// Get the current configuration (read-only)
    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// Get the primary VM directory (used for creating new VMs)
    pub async fn get_primary_vm_directory(&self) -> PathBuf {
        self.config.read().await.get_primary_vm_directory()
    }

    /// Get all VM directories for discovery
    pub async fn get_all_vm_directories(&self) -> Vec<PathBuf> {
        self.config.read().await.get_all_vm_directories().clone()
    }

    /// Set the primary VM directory
    pub async fn set_primary_vm_directory(&self, directory: PathBuf) -> Result<()> {
        {
            let mut config = self.config.write().await;
            config.set_primary_vm_directory(directory);
        }
        self.save().await
    }

    /// Add a VM directory for discovery
    pub async fn add_vm_directory(&self, directory: PathBuf) -> Result<()> {
        {
            let mut config = self.config.write().await;
            config.add_vm_directory(directory);
        }
        self.save().await
    }

    /// Remove a VM directory
    pub async fn remove_vm_directory(&self, directory: &PathBuf) -> Result<()> {
        {
            let mut config = self.config.write().await;
            config.remove_vm_directory(directory);
        }
        self.save().await
    }

    /// Update configuration settings
    pub async fn update_config<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        {
            let mut config = self.config.write().await;
            update_fn(&mut *config);
        }
        self.save().await
    }

    /// Save configuration to disk
    async fn save(&self) -> Result<()> {
        let config = self.config.read().await;
        config.save()
    }

    /// Reload configuration from disk
    pub async fn reload(&self) -> Result<()> {
        let new_config = AppConfig::load()?;
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let config_manager = ConfigManager::new().await.unwrap();
        let config = config_manager.get_config().await;

        // Should have default VM directories
        assert!(!config.vm_directories.is_empty());
    }

    #[tokio::test]
    async fn test_vm_directory_management() {
        let config_manager = ConfigManager::new().await.unwrap();

        let test_dir = PathBuf::from("/tmp/test-vms");

        // Add directory
        config_manager
            .add_vm_directory(test_dir.clone())
            .await
            .unwrap();
        let dirs = config_manager.get_all_vm_directories().await;
        assert!(dirs.contains(&test_dir));

        // Set as primary
        config_manager
            .set_primary_vm_directory(test_dir.clone())
            .await
            .unwrap();
        let primary = config_manager.get_primary_vm_directory().await;
        assert_eq!(primary, test_dir);

        // Remove directory
        config_manager.remove_vm_directory(&test_dir).await.unwrap();
        let dirs = config_manager.get_all_vm_directories().await;
        assert!(!dirs.contains(&test_dir));
    }
}
