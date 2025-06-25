mod models;
mod services;
mod ui;

use gtk::prelude::*;
use gtk::{gio, glib, Application};
use std::sync::Arc;

use models::AppConfig;
use services::{VMManager, ProcessMonitor, MetricsService, QuickgetService, ToolManager};
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
        
        // Initialize tool manager and ensure tools are available
        let tool_manager = ToolManager::new();
        println!("Checking for quickemu/quickget tools...");
        
        let (quickemu_path, quickget_path) = match tool_manager.ensure_tools_available().await {
            Ok(paths) => {
                println!("✅ Tools available");
                // Verify tools work
                if let Err(e) = tool_manager.verify_tools(&paths.0, &paths.1).await {
                    eprintln!("⚠️  Tool verification failed: {}", e);
                }
                paths
            }
            Err(e) => {
                eprintln!("❌ Failed to ensure tools are available: {}", e);
                eprintln!("VM creation will be disabled");
                (std::path::PathBuf::from("quickemu"), std::path::PathBuf::from("quickget"))
            }
        };
        
        // Check for missing dependencies
        if let Ok(missing_deps) = tool_manager.check_dependencies().await {
            if !missing_deps.is_empty() {
                eprintln!("⚠️  Missing dependencies: {}", missing_deps.join(", "));
                eprintln!("VM functionality may be limited");
            }
        }
        
        let mut vm_manager = VMManager::with_paths(quickemu_path.clone(), Some(quickget_path.clone()));
        
        let process_monitor = Arc::new(ProcessMonitor::new());
        vm_manager.set_process_monitor(process_monitor.clone());
        
        // Create quickget service if tools are properly available  
        let quickget_service = if quickget_path.exists() {
            Some(Arc::new(QuickgetService::new(quickget_path)))
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