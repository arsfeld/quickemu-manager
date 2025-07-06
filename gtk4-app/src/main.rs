mod ui;

use gtk::prelude::*;
use gtk::{gdk, gio, glib, Application};
use std::path::PathBuf;
use std::sync::Arc;

use quickemu_core::{BinaryDiscovery, ConfigManager, ProcessMonitor, QuickgetService, VMManager};
use ui::MainWindow;

// Import AppState from lib.rs instead of defining it here
use quickemu_manager_gtk::AppState;

const APP_ID: &str = "com.github.quickemu_manager";

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();

    // Initialize libadwaita
    adw::init().expect("Failed to initialize libadwaita");

    // Load resources
    gio::resources_register_include!("resources.gresource").expect("Failed to register resources");

    // Verify that the template resource exists
    let resources = gio::resources_enumerate_children(
        "/org/quickemu/Manager/ui",
        gio::ResourceLookupFlags::NONE,
    )
    .expect("Failed to enumerate resources");
    println!("Available UI resources: {:?}", resources);
    
    // Load CSS
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_resource("/org/quickemu/Manager/style.css");
    
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    // Initialize application state
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let app_state = rt.block_on(async {
        let config_manager = ConfigManager::new()
            .await
            .expect("Failed to initialize ConfigManager");

        // Use unified binary discovery
        let binary_discovery = BinaryDiscovery::new().await;
        println!(
            "Binary discovery results:\n{}",
            binary_discovery.discovery_info()
        );

        let mut vm_manager = VMManager::from_binary_discovery(binary_discovery.clone())
            .await
            .expect("Failed to initialize VMManager");
        
        // Initialize process monitor
        let process_monitor = Arc::new(ProcessMonitor::new());
        vm_manager.set_process_monitor(process_monitor.clone());

        // Initialize quickget service if available
        let quickget_service = binary_discovery
            .quickget_path()
            .map(|path| Arc::new(QuickgetService::new(path.to_path_buf())));

        if quickget_service.is_some() {
            println!("✅ Quickget service initialized");
        } else {
            println!("⚠️  Quickget not found - VM creation will be limited");
        }

        AppState {
            config_manager,
            vm_manager: Arc::new(vm_manager),
            quickget_service,
            process_monitor,
        }
    });

    // Create main window
    let window = MainWindow::new(app, app_state, rt);
    window.present();
}
