use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::OnceCell;
use std::time::{SystemTime, Duration};
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSInfo {
    pub name: String,
    pub versions: Vec<String>,
    pub editions: Option<Vec<String>>,
    pub homepage: Option<String>,
    pub png_icon: Option<String>,
    pub svg_icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuickgetCache {
    os_list: Vec<OSInfo>,
    timestamp: SystemTime,
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

    fn get_cache_path() -> PathBuf {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("quickemu-manager");
        
        // Ensure cache directory exists
        let _ = fs::create_dir_all(&cache_dir);
        
        cache_dir.join("quickget_cache.json")
    }

    fn load_cache() -> Option<QuickgetCache> {
        let cache_path = Self::get_cache_path();
        
        if cache_path.exists() {
            if let Ok(contents) = fs::read_to_string(&cache_path) {
                if let Ok(cache) = serde_json::from_str::<QuickgetCache>(&contents) {
                    // Check if cache is less than 24 hours old
                    if let Ok(elapsed) = cache.timestamp.elapsed() {
                        if elapsed < Duration::from_secs(24 * 60 * 60) {
                            return Some(cache);
                        }
                    }
                }
            }
        }
        
        None
    }

    fn save_cache(os_list: &[OSInfo]) -> Result<()> {
        let cache = QuickgetCache {
            os_list: os_list.to_vec(),
            timestamp: SystemTime::now(),
        };
        
        let cache_path = Self::get_cache_path();
        let cache_json = serde_json::to_string_pretty(&cache)?;
        
        let mut file = fs::File::create(cache_path)?;
        file.write_all(cache_json.as_bytes())?;
        
        Ok(())
    }

    pub async fn get_supported_systems(&self) -> Result<&Vec<OSInfo>> {
        self.os_cache
            .get_or_try_init(|| async {
                self.fetch_supported_systems().await
            })
            .await
    }

    async fn fetch_supported_systems(&self) -> Result<Vec<OSInfo>> {
        // Try to load from cache first
        if let Some(cache) = Self::load_cache() {
            log::info!("Loaded OS list from cache");
            return Ok(cache.os_list);
        }

        // Cache miss or expired, fetch from quickget
        log::info!("Fetching OS list from quickget...");
        let output = Command::new(&self.quickget_path)
            .arg("--list-json")
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get OS list from quickget"));
        }

        let json_str = String::from_utf8(output.stdout)?;
        
        #[derive(serde::Deserialize)]
        struct QuickgetEntry {
            #[serde(rename = "Display Name")]
            display_name: String,
            #[serde(rename = "OS")]
            os: String,
            #[serde(rename = "Release")]
            release: String,
            #[serde(rename = "Option")]
            _option: Option<String>,
            #[serde(rename = "PNG")]
            png: Option<String>,
            #[serde(rename = "SVG")]
            svg: Option<String>,
        }
        
        let entries: Vec<QuickgetEntry> = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse quickget JSON output: {}", e))?;

        // Group by OS and collect versions, icons
        let mut os_map: HashMap<String, (String, HashSet<String>, Option<String>, Option<String>)> = HashMap::new();
        
        for entry in entries {
            let entry_key = entry.os.clone();
            let (display_name, versions, png_icon, svg_icon) = os_map.entry(entry_key.clone()).or_insert_with(|| {
                (entry.display_name.clone(), std::collections::HashSet::new(), entry.png.clone(), entry.svg.clone())
            });
            versions.insert(entry.release);
        }

        let mut os_list = Vec::new();
        for (os_name, (_display_name, versions, png_icon, svg_icon)) in os_map {
            let mut versions_vec: Vec<String> = versions.into_iter().collect();
            versions_vec.sort();
            
            os_list.push(OSInfo {
                name: os_name,
                versions: versions_vec,
                editions: None, // Will be populated separately when needed
                homepage: None, // Not provided in the new format
                png_icon,
                svg_icon,
            });
        }

        // Sort by name for consistent ordering
        os_list.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Save to cache
        if let Err(e) = Self::save_cache(&os_list) {
            log::warn!("Failed to save quickget cache: {}", e);
        } else {
            log::info!("Saved OS list to cache");
        }
        
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

    pub async fn get_editions(&self, os: &str) -> Result<Vec<String>> {
        let output = Command::new(&self.quickget_path)
            .arg(os)
            .output()?;

        let output_str = String::from_utf8(output.stdout)?;
        
        // Parse the output to extract editions
        let mut editions = Vec::new();
        for line in output_str.lines() {
            if line.trim().starts_with("- Editions:") {
                // Extract editions from the line like "- Editions: Budgie COSMIC KDE..."
                let editions_line = line.trim().strip_prefix("- Editions:").unwrap_or("").trim();
                editions = editions_line.split_whitespace().map(|s| s.to_string()).collect();
                break;
            }
        }
        
        Ok(editions)
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