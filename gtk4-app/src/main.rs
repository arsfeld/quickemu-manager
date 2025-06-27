mod ui;

use gtk::prelude::*;
use gtk::{gio, glib, Application};
use std::sync::Arc;
use std::path::PathBuf;

use quickemu_core::{ConfigManager, VMManager, QuickgetService, BinaryDiscovery};
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
        let config_manager = ConfigManager::new().await
            .expect("Failed to initialize ConfigManager");
        
        // Use unified binary discovery
        let binary_discovery = BinaryDiscovery::new().await;
        println!("Binary discovery results:\n{}", binary_discovery.discovery_info());
        
        let vm_manager = VMManager::from_binary_discovery(binary_discovery.clone()).await
            .expect("Failed to initialize VMManager");

        // Initialize quickget service if available
        let quickget_service = binary_discovery.quickget_path().map(|path| {
            Arc::new(QuickgetService::new(path.to_path_buf()))
        });

        if quickget_service.is_some() {
            println!("✅ Quickget service initialized");
        } else {
            println!("⚠️  Quickget not found - VM creation will be limited");
        }

        AppState {
            config_manager,
            vm_manager: Arc::new(vm_manager),
            quickget_service,
        }
    });

    // Create main window
    let window = MainWindow::new(app, app_state, rt);
    window.present();
}
