mod ui;

use gtk::prelude::*;
use gtk::{gio, glib, Application};
use std::sync::Arc;
use std::path::PathBuf;

use quickemu_core::{AppConfig, VMManager, QuickgetService};
use ui::MainWindow;

// Import AppState from lib.rs instead of defining it here
use quickemu_manager_gtk::AppState;

const APP_ID: &str = "com.github.quickemu_manager";

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
        let vm_manager = VMManager::new().expect("Failed to initialize VMManager");

        // Try to find quickget binary and initialize service
        let quickget_service = find_quickget_binary().map(|path| {
            Arc::new(QuickgetService::new(path))
        });

        if quickget_service.is_some() {
            println!("✅ Quickget service initialized");
        } else {
            println!("⚠️  Quickget not found - VM creation will be limited");
        }

        AppState {
            config: Arc::new(tokio::sync::RwLock::new(config)),
            vm_manager: Arc::new(vm_manager),
            quickget_service,
        }
    });

    // Create main window
    let window = MainWindow::new(app, app_state, rt);
    window.present();
}

fn find_quickget_binary() -> Option<PathBuf> {
    use std::process::Command;
    
    // Try using which command to find quickget
    if let Ok(output) = Command::new("which").arg("quickget").output() {
        if output.status.success() {
            let path_string = String::from_utf8_lossy(&output.stdout);
            let path_str = path_string.trim();
            if !path_str.is_empty() {
                return Some(PathBuf::from(path_str));
            }
        }
    }
    
    // Try common locations for quickget
    let possible_paths = [
        "/usr/bin/quickget",
        "/usr/local/bin/quickget",
        "/opt/quickemu/quickget",
        "./quickget",  // Local development
    ];

    for path in &possible_paths {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() && path_buf.is_file() {
            return Some(path_buf);
        }
    }

    None
}