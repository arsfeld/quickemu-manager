use dioxus::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use {
    std::sync::Arc,
    std::path::PathBuf,
    tokio::sync::{mpsc, RwLock, Mutex},
    std::collections::HashMap,
    anyhow::Result,
    quickemu_core::{
        VMManager, VMDiscovery, QuickgetService, BinaryDiscovery,
        
        models::{VM as CoreVM, VMId, VMTemplate as CoreVMTemplate},
        DiscoveryEvent
    },
    tokio::process::Command,
    tokio::io::{AsyncBufReadExt, BufReader},
    std::process::Stdio,
    image::{DynamicImage, ImageFormat},
    base64::{engine::general_purpose::STANDARD, Engine as _},
    std::io::Cursor,
};

use crate::models::{VM, VMMetrics, CreateVMRequest};

#[cfg(not(target_arch = "wasm32"))]
use crate::server_only::*;

// Initialize services
#[cfg(not(target_arch = "wasm32"))]
pub async fn init_services() -> Result<()> {
    // Use unified binary discovery
    let binary_discovery = BinaryDiscovery::new().await;
    tracing::info!("Binary discovery results:
{}", binary_discovery.discovery_info());

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
    
    // Always refresh VM status from cache and update runtime status
    let vm_manager = VM_MANAGER.read().await;
    let mut cache = VM_CACHE.write().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        // Update runtime status for all cached VMs
        for (vm_id, vm) in cache.iter_mut() {
            // Check current runtime status
            let vm_id_obj = VMId(vm_id.clone());
            let is_running = manager.is_vm_running(&vm_id_obj).await;
            
            // Update the VM status based on actual runtime state
            vm.status = if is_running {
                quickemu_core::models::VMStatus::Running { 
                    pid: 0 // We don't have the PID here, but that's okay for display
                }
            } else {
                quickemu_core::models::VMStatus::Stopped
            };
        }
        
        let vms: Vec<VM> = cache.values().map(|core_vm| core_vm.into()).collect();
        if !vms.is_empty() {
            return Ok(vms);
        }
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
        let mut cache_update = HashMap::new();
        
        for dir in vm_dirs {
            if dir.exists() {
                match discovery.scan_directory(&dir).await {
                    Ok(vms) => {
                        for vm in vms {
                            // Add to cache
                            cache_update.insert(vm.id.0.clone(), vm.clone());
                            all_vms.push(VM::from(&vm));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scan directory {:?}: {}", dir, e);
                    }
                }
            }
        }
        
        // Update the cache with discovered VMs
        drop(discovery);
        let mut cache = VM_CACHE.write().await;
        for (id, vm) in cache_update {
            cache.insert(id, vm);
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
        
        // Update cache immediately after starting
        drop(cache);
        let mut cache = VM_CACHE.write().await;
        if let Some(vm) = cache.get_mut(&vm_id) {
            vm.status = quickemu_core::models::VMStatus::Running { 
                pid: 0 
            };
        }
        
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
        
        // Update cache immediately after stopping
        let mut cache = VM_CACHE.write().await;
        if let Some(vm) = cache.get_mut(&vm_id) {
            vm.status = quickemu_core::models::VMStatus::Stopped;
        }
        
        tracing::info!("Successfully stopped VM: {}", vm_id);
        Ok(())
    } else {
        Err(ServerFnError::new("VM Manager not initialized".to_string()))
    }
}

#[server(DeleteVM)]
pub async fn delete_vm(vm_id: String) -> Result<(), ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    tracing::info!("Attempting to delete VM: {}", vm_id);
    
    let vm_manager = VM_MANAGER.read().await;
    let cache = VM_CACHE.read().await;
    
    tracing::info!("VM Manager initialized: {}", vm_manager.is_some());
    tracing::info!("Cache contains {} VMs", cache.len());
    tracing::info!("VM {} exists in cache: {}", vm_id, cache.contains_key(&vm_id));
    
    if let (Some(ref manager), Some(vm)) = (vm_manager.as_ref(), cache.get(&vm_id)) {
        // First stop the VM if it's running
        if vm.is_running() {
            let vm_id_obj = VMId(vm_id.clone());
            if let Err(e) = manager.stop_vm(&vm_id_obj).await {
                tracing::warn!("Failed to stop VM before deletion: {}", e);
            }
        }
        
        // Delete the VM files manually
        let config_dir = vm.config_path.parent()
            .ok_or_else(|| ServerFnError::new("Invalid VM config path".to_string()))?;
        
        // Delete the VM directory and all its contents
        if config_dir.exists() {
            std::fs::remove_dir_all(config_dir).map_err(|e| {
                tracing::error!("Failed to delete VM directory {:?}: {}", config_dir, e);
                ServerFnError::new(format!("Failed to delete VM files: {}", e))
            })?;
            tracing::info!("Deleted VM directory: {:?}", config_dir);
        }
        
        // Remove from cache
        drop(cache);
        let mut cache = VM_CACHE.write().await;
        cache.remove(&vm_id);
        
        tracing::info!("Successfully deleted VM: {}", vm_id);
        Ok(())
    } else {
        Err(ServerFnError::new("VM not found or VM Manager not initialized".to_string()))
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

#[server(CreateVMWithOutput)]
pub async fn create_vm_with_output(request: CreateVMRequest) -> Result<String, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    // Use default VM directory
    let output_dir = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("VMs");
    
    // Create directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).map_err(|e| {
            ServerFnError::new(format!("Failed to create VM directory: {}", e))
        })?;
    }
    
    // Get quickget path
    let binary_discovery = BinaryDiscovery::new().await;
    let quickget_path = binary_discovery.quickget_path()
        .ok_or_else(|| ServerFnError::new("quickget not found. Please install quickemu.".to_string()))?;
    
    // Generate VM name if not provided
    let vm_name = if let Some(ref name) = request.name {
        name.clone()
    } else {
        format!("{}-{}", request.os, request.version)
    };
    
    let creation_id = format!("{}_{}", vm_name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    // Build quickget command
    let mut cmd = Command::new(quickget_path);
    cmd.current_dir(&output_dir)
        .arg(&request.os)
        .arg(&request.version)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // Add edition if specified
    if let Some(ref edition) = request.edition {
        if !edition.is_empty() {
            cmd.arg(edition);
        }
    }
    
    tracing::info!("Starting VM creation: quickget {} {} (ID: {})", request.os, request.version, creation_id);
    
    // Initialize the log for this creation process
    {
        let mut logs = VM_CREATION_LOGS.lock().await;
        logs.insert(creation_id.clone(), vec![
            format!("Starting VM creation: {} {}", request.os, request.version),
            "Executing quickget command...".to_string(),
        ]);
    }
    
    // Execute command and stream output in real-time
    let mut child = cmd.spawn().map_err(|e| {
        ServerFnError::new(format!("Failed to start quickget: {}", e))
    })?;
    
    // Stream stdout and stderr in real-time
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    
    let creation_id_stdout = creation_id.clone();
    let creation_id_stderr = creation_id.clone();
    
    // Spawn background task to handle the process and update logs
    let creation_id_bg = creation_id.clone();
    tokio::spawn(async move {
        // Read stdout
        let stdout_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                let mut logs = VM_CREATION_LOGS.lock().await;
                if let Some(log_lines) = logs.get_mut(&creation_id_stdout) {
                    log_lines.push(line);
                }
            }
        });
        
        // Read stderr
        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                let mut logs = VM_CREATION_LOGS.lock().await;
                if let Some(log_lines) = logs.get_mut(&creation_id_stderr) {
                    log_lines.push(format!("ERROR: {}", line));
                }
            }
        });
        
        // Wait for process to complete
        let status = child.wait().await;
        
        // Wait for all output to be read
        let _ = tokio::join!(stdout_task, stderr_task);
        
        // Update final status
        let mut logs = VM_CREATION_LOGS.lock().await;
        if let Some(log_lines) = logs.get_mut(&creation_id_bg) {
            match status {
                Ok(status) if status.success() => {
                    log_lines.push("✓ VM created successfully!".to_string());
                }
                Ok(status) => {
                    log_lines.push(format!("✗ quickget failed with exit code: {:?}", status.code()));
                }
                Err(e) => {
                    log_lines.push(format!("✗ Process error: {}", e));
                }
            }
        }
    });
    
    // Return immediately with the creation ID
    Ok(creation_id)
}

// TODO: Implement VM screenshot functionality
// This requires integration with QEMU's screenshot capabilities
/*
#[server(GetVMScreenshot)]
pub async fn get_vm_screenshot(vm_id: String) -> Result<String, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        let vm_id_obj = VMId(vm_id.clone());
        
        // Take screenshot (returns a DynamicImage)
        let screenshot = manager.get_vm_screenshot(&vm_id_obj).await.map_err(|e| {
            tracing::error!("Failed to get screenshot for VM {}: {}", vm_id, e);
            ServerFnError::new(format!("Failed to get screenshot: {}", e))
        })?;
        
        // Convert to PNG and then to base64
        let mut image_data = Cursor::new(Vec::new());
        screenshot.write_to(&mut image_data, ImageFormat::Png).map_err(|e| {
            ServerFnError::new(format!("Failed to encode screenshot: {}", e))
        })?;
        
        let base64_string = STANDARD.encode(image_data.into_inner());
        
        Ok(format!("data:image/png;base64,{}", base64_string))
    } else {
        Err(ServerFnError::new("VM Manager not initialized".to_string()))
    }
}
*/

#[server(GetVMScreenshot)]
pub async fn get_vm_screenshot(_vm_id: String) -> Result<String, ServerFnError> {
    // Placeholder implementation - screenshot functionality not yet implemented
    Err(ServerFnError::new("Screenshot functionality not yet implemented".to_string()))
}

#[server(GetVMCreationLogs)]
pub async fn get_vm_creation_logs(creation_id: String) -> Result<Vec<String>, ServerFnError> {
    let logs = VM_CREATION_LOGS.lock().await;
    if let Some(log_lines) = logs.get(&creation_id) {
        Ok(log_lines.clone())
    } else {
        Ok(vec!["Creation process not found".to_string()])
    }
}

#[server(CleanupVMCreationLogs)]
pub async fn cleanup_vm_creation_logs(creation_id: String) -> Result<(), ServerFnError> {
    let mut logs = VM_CREATION_LOGS.lock().await;
    logs.remove(&creation_id);
    Ok(())
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
        // Use the get_editions method to fetch editions dynamically from quickget
        match service.get_editions(&os_name).await {
            Ok(editions) => {
                tracing::info!("Retrieved {} editions for {}: {:?}", editions.len(), os_name, editions);
                Ok(editions)
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


#[server(GetOSIcon)]
pub async fn get_os_icon(os_name: String) -> Result<Option<String>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let quickget_service = QUICKGET_SERVICE.read().await;
    
    if let Some(ref service) = quickget_service.as_ref() {
        match service.get_supported_systems().await {
            Ok(systems) => {
                if let Some(os_info) = systems.iter().find(|os| os.name == os_name) {
                    Ok(os_info.png_icon.clone())
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                tracing::error!("Failed to get icon for {}: {}", os_name, e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
