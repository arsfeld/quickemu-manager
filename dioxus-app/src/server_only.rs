use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use once_cell::sync::Lazy;
use quickemu_core::{
    VMManager, VMDiscovery, QuickgetService,
    models::{VM as CoreVM, VMMetrics, VMId},
};

// Global state for server-side operations
pub static VM_MANAGER: Lazy<Arc<RwLock<Option<VMManager>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static PROCESS_MONITOR: Lazy<Arc<RwLock<Option<ProcessMonitor>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static QUICKGET_SERVICE: Lazy<Arc<RwLock<Option<QuickgetService>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static VM_DISCOVERY: Lazy<Arc<RwLock<Option<VMDiscovery>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static VM_CACHE: Lazy<Arc<RwLock<HashMap<String, CoreVM>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
pub static VM_CREATION_LOGS: Lazy<Arc<Mutex<HashMap<String, Vec<String>>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Process monitor for VM metrics
pub struct ProcessMonitor {
    // Implementation would go here
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor {}
    }

    pub async fn get_vm_metrics(&self, vm_id: &VMId) -> Option<VMMetrics> {
        // TODO: Implement actual process monitoring
        // For now, return mock metrics
        Some(VMMetrics {
            cpu_percent: 0.0,
            memory_mb: 0,
            memory_percent: 0.0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        })
    }
}