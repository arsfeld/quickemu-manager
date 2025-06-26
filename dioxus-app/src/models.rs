use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMStatus {
    Running { pid: u32 },
    Stopped,
    Starting,
    Stopping,
    Error(String),
}

impl VMStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, VMStatus::Running { .. })
    }
    
    pub fn display_text(&self) -> &str {
        match self {
            VMStatus::Running { .. } => "Running",
            VMStatus::Stopped => "Stopped",
            VMStatus::Starting => "Starting...",
            VMStatus::Stopping => "Stopping...",
            VMStatus::Error(_) => "Error",
        }
    }
    
    pub fn display_class(&self) -> &str {
        match self {
            VMStatus::Running { .. } => "status-running",
            VMStatus::Stopped => "status-stopped",
            VMStatus::Starting => "status-transitioning",
            VMStatus::Stopping => "status-transitioning",
            VMStatus::Error(_) => "status-error",
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub memory_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVMRequest {
    pub os: String,
    pub version: String,
    pub edition: Option<String>,
}