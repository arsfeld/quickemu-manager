use dioxus::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use {
    std::sync::Arc,
    std::path::PathBuf,
    tokio::sync::{mpsc, RwLock, Mutex},
    std::collections::HashMap,
    anyhow::Result,
    quickemu_core::{
        VMManager, VMDiscovery, QuickgetService, BinaryDiscovery, ProcessMonitor,
        
        models::{VM as CoreVM, VMId, VMTemplate as CoreVMTemplate},
        DiscoveryEvent,
        services::spice_proxy::{SpiceProxyConfig, SpiceProxyService},
    },
    tokio::process::Command,
    tokio::io::{AsyncBufReadExt, BufReader},
    std::process::Stdio,
    image::{DynamicImage, ImageFormat},
    base64::{engine::general_purpose::STANDARD, Engine as _},
    std::io::Cursor,
    sysinfo::System,
};

use crate::models::{VM, VMMetrics, VMMetricsHistory, CreateVMRequest, EditVMRequest, ConsoleInfo};

#[cfg(not(target_arch = "wasm32"))]
use std::fs;

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
        let monitor = Arc::new(ProcessMonitor::new());
        
        // Set the process monitor on the VM manager
        let mut vm_manager = VM_MANAGER.write().await;
        if let Some(ref mut manager) = vm_manager.as_mut() {
            manager.set_process_monitor(monitor.clone());
        }
        drop(vm_manager);
        
        *process_monitor = Some(monitor.clone());
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

    // Initialize SPICE proxy service
    let mut spice_proxy = SPICE_PROXY.write().await;
    if spice_proxy.is_none() {
        let config = SpiceProxyConfig::default();
        let mut proxy_service = SpiceProxyService::new(config);
        
        // Start the proxy service
        if let Err(e) = proxy_service.start().await {
            tracing::error!("Failed to start SPICE proxy service: {}", e);
        } else {
            tracing::info!("SPICE proxy service started successfully");
            
            // Create an Arc for the proxy service
            let proxy_arc = Arc::new(proxy_service);
            
            // Set the proxy service on the VM manager
            let mut vm_manager = VM_MANAGER.write().await;
            if let Some(ref mut manager) = vm_manager.as_mut() {
                manager.set_spice_proxy(proxy_arc.clone());
            }
            drop(vm_manager);
            
            *spice_proxy = Some(proxy_arc);
        }
    }
    drop(spice_proxy);
    
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

#[server(EditVM)]
pub async fn edit_vm(request: EditVMRequest) -> Result<(), ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    tracing::info!("Attempting to edit VM: {}", request.vm_id);
    
    let cache = VM_CACHE.read().await;
    
    if let Some(vm) = cache.get(&request.vm_id) {
        // Check if VM is running - don't allow editing running VMs
        if vm.is_running() {
            return Err(ServerFnError::new("Cannot edit a running VM. Please stop the VM first.".to_string()));
        }
        
        let config_path = PathBuf::from(&vm.config_path);
        
        if !config_path.exists() {
            return Err(ServerFnError::new("VM configuration file not found".to_string()));
        }
        
        // Read current config file
        let content = fs::read_to_string(&config_path).map_err(|e| {
            ServerFnError::new(format!("Failed to read VM config: {}", e))
        })?;
        
        let mut new_content = content;
        let mut updated = false;
        
        // Update VM name if provided
        if let Some(ref name) = request.name {
            if !name.trim().is_empty() && name != &vm.name {
                // Update guest_os line to reflect new name
                let lines: Vec<&str> = new_content.lines().collect();
                new_content = lines
                    .into_iter()
                    .map(|line| {
                        if line.trim_start().starts_with("# ") && line.contains(&vm.name) {
                            format!("# {}", name)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                updated = true;
                tracing::info!("Updated VM name from '{}' to '{}'", vm.name, name);
            }
        }
        
        // Update RAM if provided
        if let Some(ref ram) = request.ram {
            if !ram.trim().is_empty() {
                let lines: Vec<&str> = new_content.lines().collect();
                new_content = lines
                    .into_iter()
                    .map(|line| {
                        if line.trim_start().starts_with("ram=") {
                            format!("ram=\"{}\"", ram)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                updated = true;
                tracing::info!("Updated VM RAM to '{}'", ram);
            }
        }
        
        // Update CPU cores if provided
        if let Some(cpu_cores) = request.cpu_cores {
            if cpu_cores > 0 && cpu_cores != vm.config.cpu_cores {
                let lines: Vec<&str> = new_content.lines().collect();
                let mut found_cpu_line = false;
                new_content = lines
                    .into_iter()
                    .map(|line| {
                        if line.trim_start().starts_with("cpu_cores=") {
                            found_cpu_line = true;
                            format!("cpu_cores={}", cpu_cores)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                // If cpu_cores line doesn't exist, add it
                if !found_cpu_line {
                    new_content.push_str(&format!("\ncpu_cores={}", cpu_cores));
                }
                
                updated = true;
                tracing::info!("Updated VM CPU cores to {}", cpu_cores);
            }
        }
        
        // Write updated config if changes were made
        if updated {
            fs::write(&config_path, new_content).map_err(|e| {
                ServerFnError::new(format!("Failed to write VM config: {}", e))
            })?;
            
            // Update cache with new values
            drop(cache);
            let mut cache = VM_CACHE.write().await;
            if let Some(cached_vm) = cache.get_mut(&request.vm_id) {
                if let Some(ref name) = request.name {
                    if !name.trim().is_empty() {
                        cached_vm.name = name.clone();
                    }
                }
                if let Some(ref ram) = request.ram {
                    if !ram.trim().is_empty() {
                        cached_vm.config.ram = ram.clone();
                    }
                }
                if let Some(cpu_cores) = request.cpu_cores {
                    if cpu_cores > 0 {
                        cached_vm.config.cpu_cores = cpu_cores;
                    }
                }
            }
            
            tracing::info!("Successfully updated VM: {}", request.vm_id);
            Ok(())
        } else {
            tracing::info!("No changes made to VM: {}", request.vm_id);
            Ok(())
        }
    } else {
        Err(ServerFnError::new("VM not found".to_string()))
    }
}

#[server(GetVMMetrics)]
pub async fn get_vm_metrics(vm_id: String) -> Result<VMMetrics, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let process_monitor = PROCESS_MONITOR.read().await;
    
    if let Some(ref monitor) = process_monitor.as_ref() {
        // Update metrics first to get current data
        monitor.update_metrics().await;
        
        let vm_id_obj = VMId(vm_id.clone());
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
                tracing::debug!("No metrics available for VM '{}' - VM may not be running or process not tracked", vm_id);
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

#[server(GetVMMetricsHistory)]
pub async fn get_vm_metrics_history(vm_id: String) -> Result<VMMetricsHistory, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let process_monitor = PROCESS_MONITOR.read().await;
    
    if let Some(ref monitor) = process_monitor.as_ref() {
        let vm_id_obj = VMId(vm_id.clone());
        
        // For now, we'll simulate historical data by collecting current metrics
        // In a real implementation, you'd store historical data over time
        let mut timestamps = Vec::new();
        let mut cpu_history = Vec::new();
        let mut memory_history = Vec::new();
        let mut network_rx_history = Vec::new();
        let mut network_tx_history = Vec::new();
        
        // Generate some sample historical data (last 30 data points)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        for i in 0..30 {
            let timestamp = current_time - (29 - i) * 2; // 2-second intervals
            timestamps.push(timestamp);
            
            // Get current metrics for this VM
            if let Some(current_metrics) = monitor.get_vm_metrics(&vm_id_obj).await {
                // Add some variation to simulate historical data
                let time_factor = (i as f32 * 0.1).sin();
                cpu_history.push((current_metrics.cpu_percent + time_factor * 10.0).max(0.0).min(100.0));
                memory_history.push((current_metrics.memory_percent + time_factor * 5.0).max(0.0).min(100.0));
                network_rx_history.push(current_metrics.network_rx_bytes + (i as u64 * 1024));
                network_tx_history.push(current_metrics.network_tx_bytes + (i as u64 * 512));
            } else {
                // Default values if no metrics available
                cpu_history.push(0.0);
                memory_history.push(0.0);
                network_rx_history.push(0);
                network_tx_history.push(0);
            }
        }
        
        Ok(VMMetricsHistory {
            timestamps,
            cpu_history,
            memory_history,
            network_rx_history,
            network_tx_history,
        })
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
                    
                    // Ensure SPICE display protocol is set for console access
                    if let Err(e) = ensure_spice_display(&vm_name).await {
                        log_lines.push(format!("⚠ Warning: Could not set SPICE display: {}", e));
                    } else {
                        log_lines.push("✓ SPICE display protocol configured".to_string());
                    }
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

#[server(StartVMConsole)]
pub async fn start_vm_console(vm_id: String) -> Result<ConsoleInfo, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        let vm_id_obj = VMId(vm_id.clone());
        
        // Check if VM is running and supports SPICE
        let cache = VM_CACHE.read().await;
        if let Some(vm) = cache.get(&vm_id) {
            if !vm.is_running() {
                return Err(ServerFnError::new("VM is not running".to_string()));
            }
            
            // Check if VM supports console access
            if !manager.supports_console_access(vm).await {
                return Err(ServerFnError::new("VM does not support console access".to_string()));
            }
        } else {
            return Err(ServerFnError::new("VM not found".to_string()));
        }
        
        // Create console session
        match manager.create_console_session(&vm_id_obj).await {
            Ok(console_info) => {
                tracing::info!("Started console session for VM '{}': {}", vm_id, console_info.websocket_url);
                Ok(console_info.into()) // Convert from core ConsoleInfo to our model ConsoleInfo
            }
            Err(e) => {
                tracing::error!("Failed to start console session for VM '{}': {}", vm_id, e);
                Err(ServerFnError::new(format!("Failed to start console session: {}", e)))
            }
        }
    } else {
        Err(ServerFnError::new("VM manager not available".to_string()))
    }
}

#[server(StopVMConsole)]
pub async fn stop_vm_console(connection_id: String) -> Result<(), ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        match manager.remove_console_session(&connection_id).await {
            Ok(()) => {
                tracing::info!("Stopped console session: {}", connection_id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to stop console session '{}': {}", connection_id, e);
                Err(ServerFnError::new(format!("Failed to stop console session: {}", e)))
            }
        }
    } else {
        Err(ServerFnError::new("VM manager not available".to_string()))
    }
}

#[server(GetConsoleStatus)]
pub async fn get_console_status(connection_id: String) -> Result<Option<String>, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        match manager.get_console_status(&connection_id).await {
            Ok(Some(status)) => {
                let status_str = match status {
                    quickemu_core::services::spice_proxy::ConnectionStatus::Authenticating => "authenticating".to_string(),
                    quickemu_core::services::spice_proxy::ConnectionStatus::Connected => "connected".to_string(),
                    quickemu_core::services::spice_proxy::ConnectionStatus::Disconnected => "disconnected".to_string(),
                    quickemu_core::services::spice_proxy::ConnectionStatus::Error(e) => format!("error: {}", e),
                };
                Ok(Some(status_str))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::error!("Failed to get console status for '{}': {}", connection_id, e);
                Err(ServerFnError::new(format!("Failed to get console status: {}", e)))
            }
        }
    } else {
        Err(ServerFnError::new("VM manager not available".to_string()))
    }
}

#[server(SupportsConsoleAccess)]
pub async fn supports_console_access(vm_id: String) -> Result<bool, ServerFnError> {
    init_services().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let vm_manager = VM_MANAGER.read().await;
    let cache = VM_CACHE.read().await;
    
    if let Some(ref manager) = vm_manager.as_ref() {
        if let Some(vm) = cache.get(&vm_id) {
            Ok(manager.supports_console_access(vm).await)
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// Ensure a VM configuration file has SPICE display protocol set
#[cfg(not(target_arch = "wasm32"))]
async fn ensure_spice_display(vm_name: &str) -> Result<(), String> {
    // Standard quickemu directory location
    let home_dir = std::env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let quickemu_dir = PathBuf::from(home_dir).join(".config/quickemu/my-vm");
    
    let config_file = quickemu_dir.join(format!("{}.conf", vm_name));
    
    if !config_file.exists() {
        return Err(format!("Config file not found: {}", config_file.display()));
    }
    
    // Read current config
    let content = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    // Check if display_server is already set to spice
    if content.contains("display_server=\"spice\"") {
        return Ok(()); // Already configured correctly
    }
    
    // Check if display_server is set to something else
    if content.contains("display_server=") {
        // Replace existing display_server setting
        let new_content = content
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("display_server=") {
                    "display_server=\"spice\""
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        fs::write(&config_file, new_content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    } else {
        // Append display_server setting
        let new_content = format!("{}\ndisplay_server=\"spice\"\n", content);
        
        fs::write(&config_file, new_content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }
    
    Ok(())
}
