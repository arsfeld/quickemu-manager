use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VM {
    pub id: String,
    pub name: String,
    pub status: VMStatus,
    pub os: String,
    pub version: String,
    pub cpu_cores: u32,
    pub ram_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VMStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
}

impl std::fmt::Display for VMStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMStatus::Running => write!(f, "Running"),
            VMStatus::Stopped => write!(f, "Stopped"),
            VMStatus::Starting => write!(f, "Starting"),
            VMStatus::Stopping => write!(f, "Stopping"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            base_url: "http://127.0.0.1:3000".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn list_vms(&self) -> Result<Vec<VM>> {
        let url = format!("{}/api/vms", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        let api_response: ApiResponse<Vec<VM>> = response.json().await?;
        
        if api_response.success {
            Ok(api_response.data.unwrap_or_default())
        } else {
            Err(anyhow::anyhow!(api_response.error.unwrap_or("Unknown error".to_string())))
        }
    }

    pub async fn start_vm(&self, vm_id: &str) -> Result<()> {
        let url = format!("{}/api/vms/{}/start", self.base_url, vm_id);
        let response = self.client
            .post(&url)
            .send()
            .await?;

        let api_response: ApiResponse<()> = response.json().await?;
        
        if api_response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!(api_response.error.unwrap_or("Failed to start VM".to_string())))
        }
    }

    pub async fn stop_vm(&self, vm_id: &str) -> Result<()> {
        let url = format!("{}/api/vms/{}/stop", self.base_url, vm_id);
        let response = self.client
            .post(&url)
            .send()
            .await?;

        let api_response: ApiResponse<()> = response.json().await?;
        
        if api_response.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!(api_response.error.unwrap_or("Failed to stop VM".to_string())))
        }
    }

    pub async fn get_vm_status(&self, vm_id: &str) -> Result<VMStatus> {
        let url = format!("{}/api/vms/{}/status", self.base_url, vm_id);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        let api_response: ApiResponse<VMStatus> = response.json().await?;
        
        if api_response.success {
            Ok(api_response.data.unwrap_or(VMStatus::Stopped))
        } else {
            Err(anyhow::anyhow!(api_response.error.unwrap_or("Failed to get VM status".to_string())))
        }
    }
}