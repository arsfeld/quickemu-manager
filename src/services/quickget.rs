use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::OnceCell;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSInfo {
    pub name: String,
    pub versions: Vec<String>,
    pub homepage: Option<String>,
}

pub struct QuickgetService {
    quickget_path: PathBuf,
    os_cache: OnceCell<Vec<OSInfo>>,
}

impl QuickgetService {
    pub fn new(quickget_path: PathBuf) -> Self {
        Self {
            quickget_path,
            os_cache: OnceCell::new(),
        }
    }

    pub async fn get_supported_systems(&self) -> Result<&Vec<OSInfo>> {
        self.os_cache
            .get_or_try_init(|| async {
                self.fetch_supported_systems().await
            })
            .await
    }

    async fn fetch_supported_systems(&self) -> Result<Vec<OSInfo>> {
        let output = Command::new(&self.quickget_path)
            .arg("--list-json")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get OS list from quickget"));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let os_data: HashMap<String, serde_json::Value> = serde_json::from_str(&json_str)
            .map_err(|_| anyhow!("Failed to parse quickget JSON output"))?;

        let mut os_list = Vec::new();
        
        for (os_name, os_info) in os_data {
            let versions = if let Some(releases) = os_info.get("releases") {
                if let Some(releases_obj) = releases.as_object() {
                    releases_obj.keys().cloned().collect()
                } else if let Some(releases_arr) = releases.as_array() {
                    releases_arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else {
                    vec!["latest".to_string()]
                }
            } else {
                vec!["latest".to_string()]
            };

            let homepage = os_info.get("homepage")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            os_list.push(OSInfo {
                name: os_name,
                versions,
                homepage,
            });
        }

        // Sort by name for consistent ordering
        os_list.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(os_list)
    }

    pub async fn get_popular_systems(&self) -> Result<Vec<OSInfo>> {
        let all_systems = self.get_supported_systems().await?;
        
        // Define popular systems in order of preference
        let popular_names = [
            "ubuntu", "fedora", "debian", "archlinux", "manjaro", 
            "opensuse", "centos-stream", "windows", "macos", "freebsd"
        ];

        let mut popular = Vec::new();
        for name in &popular_names {
            if let Some(os) = all_systems.iter().find(|os| os.name == *name) {
                popular.push(os.clone());
            }
        }

        Ok(popular)
    }

    pub async fn check_image_url(&self, os: &str, version: &str) -> Result<String> {
        let output = Command::new(&self.quickget_path)
            .arg("--url")
            .arg(os)
            .arg(version)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get image URL: {}", stderr));
        }

        let url = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(url)
    }

    pub async fn open_homepage(&self, os: &str) -> Result<()> {
        let output = Command::new(&self.quickget_path)
            .arg("--open-homepage")
            .arg(os)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to open homepage: {}", stderr));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_quickget_service() {
        // This test requires quickget to be installed
        if let Ok(quickget_path) = which::which("quickget") {
            let service = QuickgetService::new(quickget_path);
            
            // Test getting supported systems
            let systems = service.get_supported_systems().await;
            assert!(systems.is_ok());
            
            let systems = systems.unwrap();
            assert!(!systems.is_empty());
            
            // Test getting popular systems
            let popular = service.get_popular_systems().await;
            assert!(popular.is_ok());
            
            let popular = popular.unwrap();
            assert!(!popular.is_empty());
            
            // Ubuntu should be in popular systems
            assert!(popular.iter().any(|os| os.name == "ubuntu"));
        }
    }
}