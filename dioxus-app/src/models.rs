use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use quickemu_core::models::{VM as CoreVM, VMTemplate as CoreVMTemplate};

// Re-export core types for server side only
#[cfg(not(target_arch = "wasm32"))]
pub use quickemu_core::models::{VMStatus as CoreVMStatus};

// Define our own VMStatus for cross-platform compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMStatus {
    Running { pid: u32 },
    Stopped,
    Starting,
    Stopping,
    Error(String),
}

// Define our own VMMetrics for cross-platform compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VMMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub memory_percent: f32,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

// Historical metrics for graphing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VMMetricsHistory {
    pub timestamps: Vec<u64>,
    pub cpu_history: Vec<f32>,
    pub memory_history: Vec<f32>,
    pub network_rx_history: Vec<u64>,
    pub network_tx_history: Vec<u64>,
}

// Extension trait to add UI-specific methods to VMStatus
pub trait VMStatusExt {
    fn is_running(&self) -> bool;
    fn display_text(&self) -> &str;
}

impl VMStatusExt for VMStatus {
    fn is_running(&self) -> bool {
        matches!(self, VMStatus::Running { .. })
    }
    
    fn display_text(&self) -> &str {
        match self {
            VMStatus::Running { .. } => "Running",
            VMStatus::Stopped => "Stopped",
            VMStatus::Starting => "Starting...",
            VMStatus::Stopping => "Stopping...",
            VMStatus::Error(_) => "Error",
        }
    }
}

// Simplified VM model for web UI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VM {
    pub id: String,
    pub name: String,
    pub os: String,
    pub version: String,
    pub status: VMStatus,
    pub cpu_cores: u32,
    pub ram_mb: u32,
    pub disk_size: String,
    pub config_path: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&CoreVM> for VM {
    fn from(core_vm: &CoreVM) -> Self {
        // Parse RAM from string format (e.g., "4G" -> 4096MB)
        let ram_mb = parse_ram_size(&core_vm.config.ram);
        
        // Convert core VMStatus to our VMStatus
        let status = match &core_vm.status {
            CoreVMStatus::Running { pid } => VMStatus::Running { pid: *pid },
            CoreVMStatus::Stopped => VMStatus::Stopped,
            CoreVMStatus::Starting => VMStatus::Starting,
            CoreVMStatus::Stopping => VMStatus::Stopping,
            CoreVMStatus::Error(msg) => VMStatus::Error(msg.clone()),
        };
        
        Self {
            id: core_vm.id.0.clone(),
            name: core_vm.name.clone(),
            os: core_vm.config.guest_os.clone(),
            version: extract_version(&core_vm.config.guest_os),
            status,
            cpu_cores: core_vm.config.cpu_cores,
            ram_mb,
            disk_size: core_vm.config.disk_size.clone().unwrap_or_else(|| "N/A".to_string()),
            config_path: core_vm.config_path.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVMRequest {
    pub os: String,
    pub version: String,
    pub edition: Option<String>,
    pub name: Option<String>,
    pub ram: Option<String>,
    pub disk_size: Option<String>,
    pub cpu_cores: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditVMRequest {
    pub vm_id: String,
    pub name: Option<String>,
    pub ram: Option<String>,
    pub cpu_cores: Option<u32>,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<CreateVMRequest> for CoreVMTemplate {
    fn from(request: CreateVMRequest) -> Self {
        Self {
            name: request.name.unwrap_or_else(|| format!("{} {}", request.os, request.version)),
            os: request.os,
            version: request.version,
            edition: request.edition,
            ram: request.ram.unwrap_or_else(|| "4G".to_string()),
            disk_size: request.disk_size.unwrap_or_else(|| "25G".to_string()),
            cpu_cores: request.cpu_cores.unwrap_or(4),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod helpers {
    // Helper function to parse RAM size from quickemu format
    pub fn parse_ram_size(ram_str: &str) -> u32 {
        let ram_str = ram_str.trim().to_uppercase();
        
        if let Some(gb_pos) = ram_str.find('G') {
            if let Ok(gb) = ram_str[..gb_pos].parse::<u32>() {
                return gb * 1024; // Convert GB to MB
            }
        }
        
        if let Some(mb_pos) = ram_str.find('M') {
            if let Ok(mb) = ram_str[..mb_pos].parse::<u32>() {
                return mb;
            }
        }
        
        // Default fallback
        4096 // 4GB in MB
    }

    // Helper function to extract version from guest OS string
    pub fn extract_version(guest_os: &str) -> String {
        // Try to extract version from strings like "ubuntu-22.04" or "fedora-39"
        if let Some(dash_pos) = guest_os.rfind('-') {
            guest_os[dash_pos + 1..].to_string()
        } else {
            "N/A".to_string()
        }
    }
}

// Console information for SPICE connections - cross-platform compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleInfo {
    pub websocket_url: String,
    pub auth_token: String,
    pub connection_id: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<quickemu_core::services::spice_proxy::ConsoleInfo> for ConsoleInfo {
    fn from(core_info: quickemu_core::services::spice_proxy::ConsoleInfo) -> Self {
        Self {
            websocket_url: core_info.websocket_url,
            auth_token: core_info.auth_token,
            connection_id: core_info.connection_id,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use helpers::*;

// Cross-platform configuration data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub vm_directories: Vec<String>,
    pub auto_download_tools: bool,
    pub theme: ThemeDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemeDto {
    System,
    Light,
    Dark,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<quickemu_core::models::config::AppConfig> for AppConfigDto {
    fn from(config: quickemu_core::models::config::AppConfig) -> Self {
        Self {
            vm_directories: config.vm_directories.into_iter().map(|p| p.to_string_lossy().to_string()).collect(),
            auto_download_tools: config.auto_download_tools,
            theme: match config.theme {
                quickemu_core::models::config::Theme::System => ThemeDto::System,
                quickemu_core::models::config::Theme::Light => ThemeDto::Light,
                quickemu_core::models::config::Theme::Dark => ThemeDto::Dark,
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<AppConfigDto> for quickemu_core::models::config::AppConfig {
    fn from(dto: AppConfigDto) -> Self {
        use std::path::PathBuf;
        Self {
            vm_directories: dto.vm_directories.into_iter().map(PathBuf::from).collect(),
            quickemu_path: None,
            quickget_path: None,
            auto_download_tools: dto.auto_download_tools,
            theme: match dto.theme {
                ThemeDto::System => quickemu_core::models::config::Theme::System,
                ThemeDto::Light => quickemu_core::models::config::Theme::Light,
                ThemeDto::Dark => quickemu_core::models::config::Theme::Dark,
            },
            update_interval_ms: 1000, // Fixed 1 second for file watching
        }
    }
}

impl AppConfigDto {
    pub fn get_primary_vm_directory(&self) -> String {
        self.vm_directories.first()
            .cloned()
            .unwrap_or_else(|| {
                if let Some(home) = std::env::var("HOME").ok() {
                    format!("{}/VMs", home)
                } else {
                    "/tmp/VMs".to_string()
                }
            })
    }
}