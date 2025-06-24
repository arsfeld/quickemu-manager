use crate::models::{VM, VMId, VMStatus, VMTemplate};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct VMManager {
    quickemu_path: PathBuf,
    quickget_path: Option<PathBuf>,
    processes: Arc<RwLock<HashMap<VMId, Child>>>,
}

impl VMManager {
    pub fn new() -> Result<Self> {
        let quickemu_path = which::which("quickemu").map_err(|_| {
            anyhow!("quickemu not found in PATH. Please install quickemu or configure the path.")
        })?;
        
        let quickget_path = which::which("quickget").ok();
        
        Ok(Self {
            quickemu_path,
            quickget_path,
            processes: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub fn with_paths(quickemu_path: PathBuf, quickget_path: Option<PathBuf>) -> Self {
        Self {
            quickemu_path,
            quickget_path,
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start_vm(&self, vm: &VM) -> Result<()> {
        if vm.is_running() {
            return Err(anyhow!("VM is already running"));
        }
        
        let config_dir = vm.config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid config path"))?;
        
        let mut cmd = Command::new(&self.quickemu_path);
        cmd.arg("--vm")
            .arg(&vm.config_path)
            .current_dir(config_dir);
        
        let child = cmd.spawn()?;
        let _pid = child.id();
        
        self.processes.write().await.insert(vm.id.clone(), child);
        
        Ok(())
    }
    
    pub async fn stop_vm(&self, vm_id: &VMId) -> Result<()> {
        let mut processes = self.processes.write().await;
        
        if let Some(mut child) = processes.remove(vm_id) {
            child.kill()?;
            child.wait()?;
            Ok(())
        } else {
            Err(anyhow!("VM process not found"))
        }
    }
    
    pub async fn restart_vm(&self, vm: &VM) -> Result<()> {
        if vm.is_running() {
            self.stop_vm(&vm.id).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        self.start_vm(vm).await
    }
    
    pub async fn get_vm_status(&self, vm_id: &VMId) -> VMStatus {
        let mut processes = self.processes.write().await;
        
        if let Some(child) = processes.get_mut(vm_id) {
            match child.try_wait() {
                Ok(Some(_)) => {
                    processes.remove(vm_id);
                    VMStatus::Stopped
                }
                Ok(None) => VMStatus::Running { pid: child.id() },
                Err(e) => {
                    processes.remove(vm_id);
                    VMStatus::Error(e.to_string())
                }
            }
        } else {
            VMStatus::Stopped
        }
    }
    
    pub async fn create_vm(&self, template: VMTemplate, target_dir: &Path) -> Result<PathBuf> {
        let quickget_path = self.quickget_path
            .as_ref()
            .ok_or_else(|| anyhow!("quickget not available"))?;
        
        std::fs::create_dir_all(target_dir)?;
        
        let mut cmd = Command::new(quickget_path);
        cmd.arg(&template.os)
            .arg(&template.version)
            .current_dir(target_dir);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create VM: {}", stderr));
        }
        
        let config_name = format!("{}-{}.conf", template.os, template.version);
        let config_path = target_dir.join(&config_name);
        
        if !config_path.exists() {
            return Err(anyhow!("Config file not created: {}", config_path.display()));
        }
        
        Ok(config_path)
    }
    
    pub async fn update_vm_status(&self, vm: &mut VM) {
        vm.status = self.get_vm_status(&vm.id).await;
    }
    
    pub async fn launch_display(&self, vm: &VM) -> Result<()> {
        if !vm.is_running() {
            return Err(anyhow!("VM is not running"));
        }
        
        match &vm.config.display {
            crate::models::DisplayProtocol::Spice { port } => {
                Command::new("remote-viewer")
                    .arg(format!("spice://localhost:{}", port))
                    .spawn()?;
            }
            crate::models::DisplayProtocol::Vnc { port } => {
                Command::new("vncviewer")
                    .arg(format!("localhost:{}", port))
                    .spawn()?;
            }
            _ => return Err(anyhow!("Display protocol not supported for launching")),
        }
        
        Ok(())
    }
    
    pub fn is_quickget_available(&self) -> bool {
        self.quickget_path.is_some()
    }
}