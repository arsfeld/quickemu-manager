use dioxus::prelude::*;
use crate::models::{VM, VMStatus, VMMetrics, CreateVMRequest};

#[server(GetVMs)]
pub async fn get_vms() -> Result<Vec<VM>, ServerFnError> {
    // Mock data for now - replace with actual VM discovery
    Ok(vec![
        VM {
            id: "ubuntu-22-04".to_string(),
            name: "Ubuntu 22.04".to_string(),
            os: "Ubuntu".to_string(),
            version: "22.04".to_string(),
            status: VMStatus::Running { pid: 1234 },
            cpu_cores: 4,
            ram_mb: 8192,
            disk_size: "50G".to_string(),
            config_path: "/home/user/VMs/ubuntu-22.04.conf".to_string(),
        },
        VM {
            id: "fedora-39".to_string(),
            name: "Fedora 39".to_string(),
            os: "Fedora".to_string(),
            version: "39".to_string(),
            status: VMStatus::Stopped,
            cpu_cores: 2,
            ram_mb: 4096,
            disk_size: "30G".to_string(),
            config_path: "/home/user/VMs/fedora-39.conf".to_string(),
        },
        VM {
            id: "windows-11".to_string(),
            name: "Windows 11".to_string(),
            os: "Windows".to_string(),
            version: "11".to_string(),
            status: VMStatus::Stopped,
            cpu_cores: 8,
            ram_mb: 16384,
            disk_size: "100G".to_string(),
            config_path: "/home/user/VMs/windows-11.conf".to_string(),
        },
    ])
}

#[server(StartVM)]
pub async fn start_vm(vm_id: String) -> Result<(), ServerFnError> {
    // Mock implementation - replace with actual quickemu start command
    println!("Starting VM: {}", vm_id);
    Ok(())
}

#[server(StopVM)]
pub async fn stop_vm(vm_id: String) -> Result<(), ServerFnError> {
    // Mock implementation - replace with actual quickemu stop command
    println!("Stopping VM: {}", vm_id);
    Ok(())
}

#[server(GetVMMetrics)]
pub async fn get_vm_metrics(vm_id: String) -> Result<VMMetrics, ServerFnError> {
    // Mock metrics - replace with actual system metrics
    Ok(VMMetrics {
        cpu_percent: 45.2,
        memory_mb: 2048,
        memory_percent: 25.0,
    })
}

#[server(CreateVM)]
pub async fn create_vm(request: CreateVMRequest) -> Result<String, ServerFnError> {
    // Mock implementation - replace with actual quickget integration
    println!("Creating VM: {} {}", request.os, request.version);
    Ok("new-vm-id".to_string())
}

#[server(GetAvailableOS)]
pub async fn get_available_os() -> Result<Vec<(String, Vec<String>)>, ServerFnError> {
    // Mock OS list - replace with actual quickget --list output
    Ok(vec![
        ("Ubuntu".to_string(), vec!["22.04".to_string(), "23.10".to_string(), "24.04".to_string()]),
        ("Fedora".to_string(), vec!["38".to_string(), "39".to_string(), "40".to_string()]),
        ("Windows".to_string(), vec!["10".to_string(), "11".to_string()]),
        ("macOS".to_string(), vec!["Monterey".to_string(), "Ventura".to_string(), "Sonoma".to_string()]),
    ])
}