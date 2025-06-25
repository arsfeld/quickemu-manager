mod models;
mod services;
mod ui;

use gtk::prelude::*;
use gtk::{gio, glib, Application};
use std::sync::Arc;

use models::AppConfig;
use services::{VMManager, ProcessMonitor, MetricsService, QuickgetService};
use ui::MainWindow;

const APP_ID: &str = "com.github.quickemu_manager";

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<tokio::sync::RwLock<AppConfig>>,
    pub vm_manager: Arc<VMManager>,
    pub process_monitor: Arc<ProcessMonitor>,
    pub metrics_service: Arc<MetricsService>,
    pub quickget_service: Option<Arc<QuickgetService>>,
}

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    // Initialize libadwaita
    adw::init().expect("Failed to initialize libadwaita");

    // Load resources
    gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    // Initialize application state
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let app_state = rt.block_on(async {
        let config = AppConfig::load().unwrap_or_default();
        let mut vm_manager = VMManager::new().unwrap_or_else(|_| {
            VMManager::with_paths(
                std::path::PathBuf::from("quickemu"),
                None
            )
        });
        
        let process_monitor = Arc::new(ProcessMonitor::new());
        vm_manager.set_process_monitor(process_monitor.clone());
        
        let quickget_service = if vm_manager.is_quickget_available() {
            which::which("quickget")
                .ok()
                .map(|path| Arc::new(QuickgetService::new(path)))
        } else {
            None
        };

        AppState {
            config: Arc::new(tokio::sync::RwLock::new(config)),
            vm_manager: Arc::new(vm_manager),
            process_monitor,
            metrics_service: Arc::new(MetricsService::new(60)),
            quickget_service,
        }
    });

    // Create main window
    let window = MainWindow::new(app, app_state, rt);
    window.present();
}