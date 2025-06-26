use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use quickemu_core::models::{VM as CoreVM, VMTemplate as CoreVMTemplate};

// Re-export core types for server side only
#[cfg(not(target_arch = "wasm32"))]
pub use quickemu_core::models::{VMStatus as CoreVMStatus, VMMetrics as CoreVMMetrics};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub memory_percent: f32,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
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
            disk_size: core_vm.config.disk_size.clone().unwrap_or_else(|| "Unknown".to_string()),
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
            "Unknown".to_string()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use helpers::*;