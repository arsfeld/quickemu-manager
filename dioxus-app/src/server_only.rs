use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use once_cell::sync::Lazy;
use quickemu_core::{
    VMManager, VMDiscovery, QuickgetService, ProcessMonitor, ConfigManager,
    models::{VM as CoreVM, VMMetrics, VMId},
    services::spice_proxy::SpiceProxyService,
};

// Global state for server-side operations
pub static VM_MANAGER: Lazy<Arc<RwLock<Option<VMManager>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static PROCESS_MONITOR: Lazy<Arc<RwLock<Option<Arc<ProcessMonitor>>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static QUICKGET_SERVICE: Lazy<Arc<RwLock<Option<QuickgetService>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static VM_DISCOVERY: Lazy<Arc<RwLock<Option<VMDiscovery>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static VM_CACHE: Lazy<Arc<RwLock<HashMap<String, CoreVM>>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
pub static VM_CREATION_LOGS: Lazy<Arc<Mutex<HashMap<String, Vec<String>>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static SPICE_PROXY: Lazy<Arc<RwLock<Option<Arc<SpiceProxyService>>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static CONFIG_MANAGER: Lazy<Arc<RwLock<Option<ConfigManager>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
pub static VM_CACHE_VERSION: Lazy<Arc<RwLock<u64>>> = Lazy::new(|| Arc::new(RwLock::new(0)));

