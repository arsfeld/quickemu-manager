use crate::models::{VM, VMId, VMStatus};
use crate::services::{ConfigParser, VMManager};
use anyhow::Result;
use gtk::glib;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub enum DiscoveryEvent {
    VMAdded(VM),
    VMUpdated(VM),
    VMRemoved(VMId),
}

pub struct VMDiscovery {
    vms: Arc<RwLock<HashMap<VMId, VM>>>,
    watched_dirs: Vec<PathBuf>,
    event_tx: mpsc::UnboundedSender<DiscoveryEvent>,
    vm_manager: Option<Arc<VMManager>>,
}

impl VMDiscovery {
    pub fn new(event_tx: mpsc::UnboundedSender<DiscoveryEvent>) -> Self {
        Self {
            vms: Arc::new(RwLock::new(HashMap::new())),
            watched_dirs: Vec::new(),
            event_tx,
            vm_manager: None,
        }
    }
    
    pub fn with_vm_manager(event_tx: mpsc::UnboundedSender<DiscoveryEvent>, vm_manager: Arc<VMManager>) -> Self {
        Self {
            vms: Arc::new(RwLock::new(HashMap::new())),
            watched_dirs: Vec::new(),
            event_tx,
            vm_manager: Some(vm_manager),
        }
    }
    
    pub async fn scan_directory(&mut self, path: &Path) -> Result<Vec<VM>> {
        let mut discovered_vms = Vec::new();
        
        if !path.exists() {
            return Ok(discovered_vms);
        }
        
        // Use a stack-based approach to avoid async recursion
        let mut directories_to_scan = vec![path.to_path_buf()];
        
        while let Some(current_dir) = directories_to_scan.pop() {
            let entries = std::fs::read_dir(&current_dir)?;
            
            for entry in entries {
                let entry = entry?;
                let entry_path = entry.path();
                
                if entry_path.is_file() && entry_path.extension().and_then(|s| s.to_str()) == Some("conf") {
                    if let Ok(vm) = self.load_vm_from_config(&entry_path).await {
                        discovered_vms.push(vm.clone());
                        self.vms.write().await.insert(vm.id.clone(), vm);
                    }
                } else if entry_path.is_dir() {
                    // Add subdirectory to scan queue
                    directories_to_scan.push(entry_path);
                }
            }
        }
        
        if !self.watched_dirs.contains(&path.to_path_buf()) {
            self.watched_dirs.push(path.to_path_buf());
        }
        
        Ok(discovered_vms)
    }
    
    async fn load_vm_from_config(&self, config_path: &Path) -> Result<VM> {
        let config = ConfigParser::parse_quickemu_config(config_path)?;
        let name = config_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let id = VMId(name.clone());
        let metadata = std::fs::metadata(config_path)?;
        let last_modified = metadata.modified()?;
        
        let mut vm = VM {
            id,
            name,
            config_path: config_path.to_path_buf(),
            config,
            status: VMStatus::Stopped,
            last_modified,
        };
        
        // Check if VM is running
        if let Some(vm_manager) = &self.vm_manager {
            vm_manager.update_vm_status_from_running_processes(&mut vm).await;
        }
        
        Ok(vm)
    }
    
    pub async fn start_watching(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )?;
        
        for dir in &self.watched_dirs {
            watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }
        
        let vms = self.vms.clone();
        let event_tx = self.event_tx.clone();
        let vm_manager = self.vm_manager.clone();
        
        glib::spawn_future_local(async move {
            while let Some(event) = rx.recv().await {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if path.extension().and_then(|s| s.to_str()) == Some("conf") {
                                if let Ok(vm) = Self::load_vm_from_config_static(&path, vm_manager.as_ref()).await {
                                    let mut vms_lock = vms.write().await;
                                    let is_new = !vms_lock.contains_key(&vm.id);
                                    vms_lock.insert(vm.id.clone(), vm.clone());
                                    
                                    let event = if is_new {
                                        DiscoveryEvent::VMAdded(vm)
                                    } else {
                                        DiscoveryEvent::VMUpdated(vm)
                                    };
                                    
                                    let _ = event_tx.send(event);
                                }
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if path.extension().and_then(|s| s.to_str()) == Some("conf") {
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                                
                                let id = VMId(name);
                                let mut vms_lock = vms.write().await;
                                
                                if vms_lock.remove(&id).is_some() {
                                    let _ = event_tx.send(DiscoveryEvent::VMRemoved(id));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        
        Ok(())
    }
    
    async fn load_vm_from_config_static(config_path: &Path, vm_manager: Option<&Arc<VMManager>>) -> Result<VM> {
        let config = ConfigParser::parse_quickemu_config(config_path)?;
        let name = config_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let id = VMId(name.clone());
        let metadata = std::fs::metadata(config_path)?;
        let last_modified = metadata.modified()?;
        
        let mut vm = VM {
            id,
            name,
            config_path: config_path.to_path_buf(),
            config,
            status: VMStatus::Stopped,
            last_modified,
        };
        
        // Check if VM is running
        if let Some(vm_manager) = vm_manager {
            vm_manager.update_vm_status_from_running_processes(&mut vm).await;
        }
        
        Ok(vm)
    }
    
    pub async fn get_all_vms(&self) -> Vec<VM> {
        self.vms.read().await.values().cloned().collect()
    }
    
    pub async fn get_vm(&self, id: &VMId) -> Option<VM> {
        self.vms.read().await.get(id).cloned()
    }
}