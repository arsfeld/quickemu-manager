use crate::models::{VMId, VMMetrics};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::sync::RwLock;

pub struct ProcessMonitor {
    system: Arc<RwLock<System>>,
    vm_processes: Arc<RwLock<HashMap<VMId, u32>>>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            system: Arc::new(RwLock::new(System::new_all())),
            vm_processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_vm_process(&self, vm_id: VMId, pid: u32) {
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

        let system = self.system.read().await;
        let process = system.process(Pid::from(*pid as usize))?;

        let total_memory = system.total_memory();
        let memory_mb = (process.memory() / 1024) as u32;
        let memory_percent = (process.memory() as f32 / total_memory as f32) * 100.0;

        Some(VMMetrics {
            cpu_percent: process.cpu_usage(),
            memory_mb,
            memory_percent,
            disk_read_bytes: process.disk_usage().read_bytes,
            disk_write_bytes: process.disk_usage().written_bytes,
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
