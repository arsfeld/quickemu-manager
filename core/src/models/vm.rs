use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VMId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMStatus {
    Running { pid: u32 },
    Stopped,
    Starting,
    Stopping,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DisplayProtocol {
    Spice { port: u16 },
    Vnc { port: u16 },
    Sdl,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VM {
    pub id: VMId,
    pub name: String,
    pub config_path: PathBuf,
    pub config: VMConfig,
    pub status: VMStatus,
    pub last_modified: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VMConfig {
    pub guest_os: String,
    pub disk_img: Option<PathBuf>,
    pub iso: Option<PathBuf>,
    pub ram: String,
    pub cpu_cores: u32,
    pub disk_size: Option<String>,
    pub display: DisplayProtocol,
    pub ssh_port: Option<u16>,
    pub raw_config: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VMTemplate {
    pub name: String,
    pub os: String,
    pub version: String,
    pub edition: Option<String>,
    pub ram: String,
    pub disk_size: String,
    pub cpu_cores: u32,
}

impl VM {
    pub fn is_running(&self) -> bool {
        matches!(self.status, VMStatus::Running { .. })
    }

    pub fn get_display_url(&self) -> Option<String> {
        match &self.config.display {
            DisplayProtocol::Spice { port } => Some(format!("spice://localhost:{}", port)),
            DisplayProtocol::Vnc { port } => Some(format!("vnc://localhost:{}", port)),
            _ => None,
        }
    }
}
