pub mod ui;
pub mod utils;

use std::sync::Arc;

pub use quickemu_core::*;

#[derive(Clone)]
pub struct AppState {
    pub config_manager: ConfigManager,
    pub vm_manager: Arc<VMManager>,
    pub quickget_service: Option<Arc<QuickgetService>>,
    pub process_monitor: Arc<ProcessMonitor>,
}
