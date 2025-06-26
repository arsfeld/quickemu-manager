use dioxus::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use {
    std::sync::Arc,
    std::path::PathBuf,
    tokio::sync::{mpsc, RwLock},
    std::collections::HashMap,
    anyhow::Result,
    quickemu_core::{
        VMManager, VMDiscovery, QuickgetService, BinaryDiscovery,
        services::process_monitor::ProcessMonitor,
        models::{VM as CoreVM, VMId, VMTemplate as CoreVMTemplate},
        DiscoveryEvent
    },
};

use crate::models::{VM, VMMetrics, CreateVMRequest};

#[cfg(not(target_arch = "wasm32"))]
mod server_only {
    use super::*;
    
    // Global state for VM services
    pub static VM_MANAGER: once_cell::sync::Lazy<Arc<RwLock<Option<VMManager>>>> = 
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

    pub static VM_DISCOVERY: once_cell::sync::Lazy<Arc<RwLock<Option<VMDiscovery>>>> = 
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

    pub static QUICKGET_SERVICE: once_cell::sync::Lazy<Arc<RwLock<Option<QuickgetService>>>> = 
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

    pub static PROCESS_MONITOR: once_cell::sync::Lazy<Arc<RwLock<Option<ProcessMonitor>>>> = 
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

    pub static VM_CACHE: once_cell::sync::Lazy<Arc<RwLock<HashMap<String, CoreVM>>>> = 
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
}

#[cfg(not(target_arch = "wasm32"))]
use server_only::*;

// Initialize services
#[cfg(not(target_arch = "wasm32"))]
async fn init_services() -> Result<()> {
    // Use unified binary discovery
    let binary_discovery = BinaryDiscovery::new().await;
    tracing::info!("Binary discovery results:\n{}", binary_discovery.discovery_info());

    let mut vm_manager = VM_MANAGER.write().await;
    if vm_manager.is_none() {
        *vm_manager = Some(VMManager::from_binary_discovery(binary_discovery.clone()).await?);
    }
    drop(vm_manager);

    let mut process_monitor = PROCESS_MONITOR.write().await;
    if process_monitor.is_none() {
        *process_monitor = Some(ProcessMonitor::new());
    }
    drop(process_monitor);

    let mut quickget_service = QUICKGET_SERVICE.write().await;
    if quickget_service.is_none() {
        // Use binary discovery for quickget
        if let Some(quickget_path) = binary_discovery.quickget_path() {
            tracing::info!("Found quickget at: {:?}", quickget_path);
            *quickget_service = Some(QuickgetService::new(quickget_path.to_path_buf()));
        } else {
            tracing::warn!("quickget not found. VM creation will use fallback OS list.");
        }
    }
    drop(quickget_service);

    let mut vm_discovery = VM_DISCOVERY.write().await;
    if vm_discovery.is_none() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let vm_manager_ref = VM_MANAGER.read().await.clone().unwrap();
        let discovery = VMDiscovery::with_vm_manager(event_tx, Arc::new(vm_manager_ref));
        
        // Spawn background task to handle discovery events
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let mut cache = VM_CACHE.write().await;
                match event {
                    DiscoveryEvent::VMAdded(vm) => {
                        cache.insert(vm.id.0.clone(), vm);
                    }
                    DiscoveryEvent::VMUpdated(vm) => {
                        cache.insert(vm.id.0.clone(), vm);
                    }
                    DiscoveryEvent::VMRemoved(vm_id) => {
                        cache.remove(&vm_id.0);
                    }
                }
            }
        });
        
        *vm_discovery = Some(discovery);
    }
    
    Ok(())
}

#[server(GetVMs)]
pub async fn get_vms() -> Result<Vec<VM>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    // Try to get VMs from cache first
    let cache = VM_CACHE.read().await;
    if !cache.is_empty() {
        let vms: Vec<VM> = cache.values().map(|core_vm| core_vm.into()).collect();
        return Ok(vms);
    }
    drop(cache);
    
    // If cache is empty, scan default VM directories
    let mut discovery = VM_DISCOVERY.write().await;
    if let Some(ref mut discovery) = discovery.as_mut() {
        // Scan common VM directories
        let vm_dirs = vec![
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("VMs"),
            PathBuf::from("/home").join(std::env::var("USER").unwrap_or_default()).join("quickemu"),
            PathBuf::from("/opt/quickemu/vms"),
        ];
        
        let mut all_vms = Vec::new();
        for dir in vm_dirs {
            if dir.exists() {
                match discovery.scan_directory(&dir).await {
                    Ok(vms) => {
                        for vm in vms {
                            all_vms.push(VM::from(&vm));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scan directory {:?}: {}", dir, e);
                    }
                }
            }
        }
        
        Ok(all_vms)
    } else {
        Err(ServerFnError::new("VM Discovery service not initialized".to_string()))
    }
}

#[server(StartVM)]
pub async fn start_vm(vm_id: String) -> Result<(), ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    let cache = VM_CACHE.read().await;
    
    if let (Some(ref manager), Some(vm)) = (vm_manager.as_ref(), cache.get(&vm_id)) {
        manager.start_vm(vm).await.map_err(|e| {
            tracing::error!("Failed to start VM {}: {}", vm_id, e);
            ServerFnError::new(format!("Failed to start VM: {}", e))
        })?;
        
        tracing::info!("Successfully started VM: {}", vm_id);
        Ok(())
    } else {
        Err(ServerFnError::new("VM not found or VM Manager not initialized".to_string()))
    }
}

#[server(StopVM)]
pub async fn stop_vm(vm_id: String) -> Result<(), ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        let vm_id_obj = VMId(vm_id.clone());
        manager.stop_vm(&vm_id_obj).await.map_err(|e| {
            tracing::error!("Failed to stop VM {}: {}", vm_id, e);
            ServerFnError::new(format!("Failed to stop VM: {}", e))
        })?;
        
        tracing::info!("Successfully stopped VM: {}", vm_id);
        Ok(())
    } else {
        Err(ServerFnError::new("VM Manager not initialized".to_string()))
    }
}

#[server(GetVMMetrics)]
pub async fn get_vm_metrics(vm_id: String) -> Result<VMMetrics, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let process_monitor = PROCESS_MONITOR.read().await;
    
    if let Some(ref monitor) = process_monitor.as_ref() {
        let vm_id_obj = VMId(vm_id);
        match monitor.get_vm_metrics(&vm_id_obj).await {
            Some(core_metrics) => {
                // Convert from core metrics to our metrics
                Ok(VMMetrics {
                    cpu_percent: core_metrics.cpu_percent,
                    memory_mb: core_metrics.memory_mb,
                    memory_percent: core_metrics.memory_percent,
                    disk_read_bytes: core_metrics.disk_read_bytes,
                    disk_write_bytes: core_metrics.disk_write_bytes,
                    network_rx_bytes: core_metrics.network_rx_bytes,
                    network_tx_bytes: core_metrics.network_tx_bytes,
                })
            }
            None => {
                tracing::warn!("No metrics available for VM");
                // Return default metrics if monitoring fails
                Ok(VMMetrics {
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
    } else {
        Err(ServerFnError::new("Process Monitor not initialized".to_string()))
    }
}

#[server(CreateVM)]
pub async fn create_vm(request: CreateVMRequest) -> Result<String, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        let template: CoreVMTemplate = request.into();
        
        // Use default VM directory
        let output_dir = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("VMs");
        
        // Create directory if it doesn't exist
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir).map_err(|e| {
                ServerFnError::new(format!("Failed to create VM directory: {}", e))
            })?;
        }
        
        match manager.create_vm_from_template(&template, &output_dir).await {
            Ok(config_path) => {
                let vm_id = config_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("new-vm")
                    .to_string();
                
                tracing::info!("Successfully created VM: {} at {:?}", vm_id, config_path);
                Ok(vm_id)
            }
            Err(e) => {
                tracing::error!("Failed to create VM: {}", e);
                
                // Check if this is a quickget-related error
                if e.to_string().contains("No such file") || e.to_string().contains("quickget") {
                    Err(ServerFnError::new(
                        "VM creation failed: quickget not found. Please install quickemu to create VMs.".to_string()
                    ))
                } else {
                    Err(ServerFnError::new(format!("Failed to create VM: {}", e)))
                }
            }
        }
    } else {
        Err(ServerFnError::new("VM Manager not initialized".to_string()))
    }
}

#[server(GetAvailableOS)]
pub async fn get_available_os() -> Result<Vec<(String, Vec<String>)>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let quickget_service = QUICKGET_SERVICE.read().await;
    
    if let Some(ref service) = quickget_service.as_ref() {
        match service.get_supported_systems().await {
            Ok(systems) => {
                let result: Vec<(String, Vec<String>)> = systems
                    .iter()
                    .map(|system| (system.name.clone(), system.versions.clone()))
                    .collect();
                
                tracing::info!("Retrieved {} operating systems from quickget", result.len());
                Ok(result)
            }
            Err(e) => {
                tracing::error!("Failed to get supported systems from quickget: {}", e);
                Err(ServerFnError::new(format!("Failed to get OS list from quickget: {}", e)))
            }
        }
    } else {
        Err(ServerFnError::new("Quickget service not available. Please ensure quickemu is installed.".to_string()))
    }
}

#[server(GetPopularOS)]
pub async fn get_popular_os() -> Result<Vec<(String, Vec<String>)>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let quickget_service = QUICKGET_SERVICE.read().await;
    
    if let Some(ref service) = quickget_service.as_ref() {
        match service.get_popular_systems().await {
            Ok(systems) => {
                let result: Vec<(String, Vec<String>)> = systems
                    .iter()
                    .map(|system| (system.name.clone(), system.versions.clone()))
                    .collect();
                
                tracing::info!("Retrieved {} popular operating systems from quickget", result.len());
                Ok(result)
            }
            Err(e) => {
                tracing::error!("Failed to get popular systems from quickget: {}", e);
                Err(ServerFnError::new(format!("Failed to get popular OS list from quickget: {}", e)))
            }
        }
    } else {
        Err(ServerFnError::new("Quickget service not available. Please ensure quickemu is installed.".to_string()))
    }
}

#[server(GetOSEditions)]
pub async fn get_os_editions(os_name: String, version: String) -> Result<Vec<String>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let quickget_service = QUICKGET_SERVICE.read().await;
    
    if let Some(ref service) = quickget_service.as_ref() {
        // Get all supported systems and find editions for the specific OS/version
        match service.get_supported_systems().await {
            Ok(systems) => {
                if let Some(os_info) = systems.iter().find(|os| os.name == os_name) {
                    // For now, return empty editions since the current OSInfo doesn't store them
                    // In a future enhancement, we could extend the core service to parse editions
                    Ok(os_info.editions.clone().unwrap_or_default())
                } else {
                    Ok(vec![])
                }
            }
            Err(e) => {
                tracing::error!("Failed to get editions for {}/{}: {}", os_name, version, e);
                Ok(vec![]) // Return empty editions rather than failing
            }
        }
    } else {
        Err(ServerFnError::new("Quickget service not available. Please ensure quickemu is installed.".to_string()))
    }
}

