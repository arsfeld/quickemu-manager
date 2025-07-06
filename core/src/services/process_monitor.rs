use crate::models::{VMId, VMMetrics};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::RwLock;

pub struct ProcessMonitor {
    system: Arc<RwLock<System>>,
    vm_processes: Arc<RwLock<HashMap<VMId, u32>>>,
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self {
            system: Arc::new(RwLock::new(System::new_all())),
            vm_processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_vm_process(&self, vm_id: VMId, pid: u32) {
        println!("ProcessMonitor: Registering VM '{}' with PID {}", vm_id.0, pid);
        self.vm_processes.write().await.insert(vm_id, pid);
    }

    pub async fn unregister_vm_process(&self, vm_id: &VMId) {
        self.vm_processes.write().await.remove(vm_id);
    }

    pub async fn update_metrics(&self) {
        let mut system = self.system.write().await;
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            false,
            ProcessRefreshKind::everything(),
        );
        system.refresh_memory();
        system.refresh_cpu_usage();
    }

    pub async fn get_vm_metrics(&self, vm_id: &VMId) -> Option<VMMetrics> {
        let processes = self.vm_processes.read().await;
        let pid = processes.get(vm_id)?;
        println!("ProcessMonitor: Getting metrics for VM '{}' with PID {}", vm_id.0, pid);

        let system = self.system.read().await;
        let process = system.process(Pid::from(*pid as usize))?;
        
        // Log process details for debugging
        if let Some(cmd) = process.cmd().first() {
            println!("ProcessMonitor: Process {} command: {}", pid, cmd.to_string_lossy());
        }

        let total_memory = system.total_memory();
        let process_memory_kb = process.memory();
        // process.memory() returns KB, convert to MB
        let memory_mb = (process_memory_kb / 1024) as u32;
        // total_memory is also in KB
        let memory_percent = (process_memory_kb as f32 / total_memory as f32) * 100.0;
        let cpu_usage = process.cpu_usage();
        let disk_usage = process.disk_usage();
        
        println!(
            "ProcessMonitor: VM '{}' raw memory: {}KB, calculated: {}MB, total system: {}KB",
            vm_id.0, process_memory_kb, memory_mb, total_memory
        );
        println!(
            "ProcessMonitor: VM '{}' metrics - CPU: {:.1}%, Memory: {}MB ({:.1}%), Disk R/W: {}/{}",
            vm_id.0, cpu_usage, memory_mb, memory_percent, disk_usage.read_bytes, disk_usage.written_bytes
        );

        Some(VMMetrics {
            cpu_percent: cpu_usage,
            memory_mb,
            memory_percent,
            disk_read_bytes: disk_usage.read_bytes,
            disk_write_bytes: disk_usage.written_bytes,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        })
    }

    pub async fn get_all_metrics(&self) -> HashMap<VMId, VMMetrics> {
        let mut metrics = HashMap::new();
        let vm_processes = self.vm_processes.read().await.clone();

        for (vm_id, _) in vm_processes {
            if let Some(vm_metrics) = self.get_vm_metrics(&vm_id).await {
                metrics.insert(vm_id, vm_metrics);
            }
        }

        metrics
    }

    pub async fn get_system_metrics(&self) -> (f32, f32) {
        let system = self.system.read().await;
        let cpu_usage = system.global_cpu_usage();
        let memory_usage = (system.used_memory() as f32 / system.total_memory() as f32) * 100.0;

        (cpu_usage, memory_usage)
    }

    pub async fn cleanup_stale_processes(&self) {
        let system = self.system.read().await;
        let mut vm_processes = self.vm_processes.write().await;

        vm_processes.retain(|_, &mut pid| system.process(Pid::from(pid as usize)).is_some());
    }
}
