pub mod models;
pub mod services;
pub mod ui;
pub mod utils;

use std::sync::Arc;
use models::AppConfig;
use services::{VMManager, ProcessMonitor, MetricsService, QuickgetService};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<tokio::sync::RwLock<AppConfig>>,
    pub vm_manager: Arc<VMManager>,
    pub process_monitor: Arc<ProcessMonitor>,
    pub metrics_service: Arc<MetricsService>,
    pub quickget_service: Option<Arc<QuickgetService>>,
}

pub use models::*;
pub use services::*;