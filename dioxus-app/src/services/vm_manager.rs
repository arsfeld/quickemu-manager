use anyhow::Result;
use quickemu_core::{VM, VMStatus};

#[cfg(feature = "desktop")]
pub struct DesktopVMManager {
    inner: quickemu_core::VMManager,
}

#[cfg(feature = "desktop")]
impl DesktopVMManager {
    pub fn new() -> Self {
        Self {
            inner: quickemu_core::VMManager::new(),
        }
    }

    pub async fn list_vms(&self) -> Result<Vec<VM>> {
        self.inner.list_vms().await
    }

    pub async fn start_vm(&self, id: &str) -> Result<()> {
        self.inner.start_vm(id).await
    }

    pub async fn stop_vm(&self, id: &str) -> Result<()> {
        self.inner.stop_vm(id).await
    }

    pub async fn restart_vm(&self, id: &str) -> Result<()> {
        self.inner.restart_vm(id).await
    }

    pub async fn get_vm(&self, id: &str) -> Result<VM> {
        self.inner.get_vm(id).await
    }

    pub async fn get_vm_status(&self, id: &str) -> Result<VMStatus> {
        self.inner.get_vm_status(id).await
    }
}

#[cfg(feature = "server")]
pub struct ServerVMManager {
    inner: quickemu_core::VMManager,
}

#[cfg(feature = "server")]
impl ServerVMManager {
    pub fn new() -> Self {
        Self {
            inner: quickemu_core::VMManager::new(),
        }
    }

    pub async fn list_vms(&self) -> Result<Vec<VM>> {
        self.inner.list_vms().await
    }

    pub async fn start_vm(&self, id: &str) -> Result<()> {
        self.inner.start_vm(id).await
    }

    pub async fn stop_vm(&self, id: &str) -> Result<()> {
        self.inner.stop_vm(id).await
    }

    pub async fn restart_vm(&self, id: &str) -> Result<()> {
        self.inner.restart_vm(id).await
    }

    pub async fn get_vm(&self, id: &str) -> Result<VM> {
        self.inner.get_vm(id).await
    }

    pub async fn get_vm_status(&self, id: &str) -> Result<VMStatus> {
        self.inner.get_vm_status(id).await
    }
}