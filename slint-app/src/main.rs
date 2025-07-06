use anyhow::Result;
use quickemu_core::{VMManager, VMStatus, VMDiscovery, VMId, ConfigManager};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

slint::include_modules!();

fn detect_platform_style() -> &'static str {
    // Detect the platform and return appropriate style
    if cfg!(target_os = "macos") {
        "cupertino"  // macOS native style
    } else if cfg!(target_os = "windows") {
        "fluent"     // Windows 11 style
    } else if cfg!(target_os = "linux") {
        // On Linux, we could detect the desktop environment
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            match desktop.to_lowercase().as_str() {
                "gnome" | "unity" | "x-cinnamon" => "material",  // Material Design for GNOME-based
                "kde" | "plasma" => "fluent",                     // Fluent for KDE (similar to Breeze)
                "cosmic" => "cosmic",                             // COSMIC desktop style
                _ => "native"                                     // Platform default
            }
        } else {
            "native"  // Fallback to platform default
        }
    } else {
        "native"      // Default for unknown platforms
    }
}

struct AppState {
    vm_manager: Arc<VMManager>,
    vm_discovery: Arc<RwLock<VMDiscovery>>,
    config_manager: Arc<ConfigManager>,
    ui: MainWindow,
}

impl AppState {
    async fn new(ui: MainWindow) -> Result<Self> {
        let vm_manager = Arc::new(VMManager::new().await?);
        let config_manager = Arc::new(ConfigManager::new().await?);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let vm_discovery = Arc::new(RwLock::new(VMDiscovery::with_vm_manager(event_tx, vm_manager.clone())));
        
        // Load VM directories from config
        {
            let mut discovery = vm_discovery.write().await;
            let config = config_manager.get_config().await;
            
            // Add configured directories to watch
            for dir in &config.vm_directories {
                discovery.add_watch_directory(dir.clone());
            }
            
            // Scan all directories
            discovery.scan_all_directories().await?;
        }
        
        Ok(Self { vm_manager, vm_discovery, config_manager, ui })
    }

    async fn refresh_vms(&self) -> Result<()> {
        // Re-scan directories to pick up any new VMs
        {
            let mut discovery = self.vm_discovery.write().await;
            discovery.scan_all_directories().await?;
        }
        
        let discovery = self.vm_discovery.read().await;
        let vms = discovery.get_all_vms().await;
        
        let vm_infos: Vec<VmInfo> = vms.into_iter().map(|vm| {
            let (status_str, cpu_usage, ram_usage, disk_io): (&str, f64, &str, &str) = match &vm.status {
                VMStatus::Stopped => ("stopped", 0.0, "0 MB", "0 B/s"),
                VMStatus::Running { pid } => {
                    ("running", 0.0, "0 MB", "0 B/s")
                },
                VMStatus::Starting => ("starting", 0.0, "0 MB", "0 B/s"),
                VMStatus::Stopping => ("stopping", 0.0, "0 MB", "0 B/s"),
                VMStatus::Error(_) => ("error", 0.0, "0 MB", "0 B/s"),
            };
            
            VmInfo {
                id: vm.id.0.clone().into(),
                name: vm.name.clone().into(),
                os_type: vm.config.guest_os.clone().into(),
                status: status_str.into(),
                cpu_usage: cpu_usage as f32,
                ram_usage: ram_usage.into(),
                disk_io: disk_io.into(),
            }
        }).collect();
        
        let model = slint::ModelRc::from(vm_infos.as_slice());
        self.ui.set_vms(model);
        Ok(())
    }

    async fn start_vm(&self, vm_id: &str) -> Result<()> {
        let discovery = self.vm_discovery.read().await;
        if let Some(vm) = discovery.get_vm(&VMId(vm_id.to_string())).await {
            drop(discovery);
            self.vm_manager.start_vm(&vm).await?;
        }
        Ok(())
    }

    async fn stop_vm(&self, vm_id: &str) -> Result<()> {
        self.vm_manager.stop_vm(&VMId(vm_id.to_string())).await?;
        Ok(())
    }
}

fn format_memory(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{} MB", bytes / MB)
    }
}

fn format_bytes_per_sec(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    
    if bytes >= GB {
        format!("{:.1} GB/s", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB/s", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB/s", bytes as f64 / KB as f64)
    } else {
        format!("{} B/s", bytes)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();
    
    // Configure Slint backend and style
    // Options: "fluent" (Windows 11 style), "cupertino" (macOS style), "material" (Material Design), "native" (platform default)
    // You can also use "cosmic" on Linux for COSMIC desktop style
    std::env::set_var("SLINT_STYLE", detect_platform_style());

    // Create UI
    let ui = MainWindow::new()?;
    let app_state = Arc::new(AppState::new(ui.clone_strong()).await?);
    
    // Set up callbacks
    {
        let state = app_state.clone();
        let ui_handle = ui.as_weak();
        ui.on_refresh_vms(move || {
            let state = state.clone();
            slint::spawn_local(async move {
                if let Err(e) = state.refresh_vms().await {
                    eprintln!("Failed to refresh VMs: {}", e);
                }
            }).unwrap();
        });
    }
    
    {
        let state = app_state.clone();
        ui.on_start_vm(move |vm_id| {
            let state = state.clone();
            let vm_id = vm_id.to_string();
            slint::spawn_local(async move {
                if let Err(e) = state.start_vm(&vm_id).await {
                    eprintln!("Failed to start VM: {}", e);
                }
                // Refresh after starting
                if let Err(e) = state.refresh_vms().await {
                    eprintln!("Failed to refresh VMs: {}", e);
                }
            }).unwrap();
        });
    }
    
    {
        let state = app_state.clone();
        ui.on_stop_vm(move |vm_id| {
            let state = state.clone();
            let vm_id = vm_id.to_string();
            slint::spawn_local(async move {
                if let Err(e) = state.stop_vm(&vm_id).await {
                    eprintln!("Failed to stop VM: {}", e);
                }
                // Refresh after stopping
                if let Err(e) = state.refresh_vms().await {
                    eprintln!("Failed to refresh VMs: {}", e);
                }
            }).unwrap();
        });
    }
    
    ui.on_open_console(move |vm_id| {
        println!("Opening console for VM: {}", vm_id);
        // TODO: Implement console view
    });
    
    ui.on_create_vm(move || {
        println!("Create VM clicked");
        // TODO: Implement VM creation dialog
    });
    
    ui.on_show_settings(move || {
        println!("Settings clicked");
        // TODO: Implement settings dialog
    });
    
    ui.on_show_about(move || {
        println!("About clicked");
        // TODO: Implement about dialog
    });
    
    ui.on_set_theme(move |theme| {
        // This would be called if we need to set theme from UI
        // For now, theme toggle is handled directly in Slint
        println!("Theme set to: {}", theme);
    });
    
    ui.on_apply_ui_style(move |style| {
        println!("Applying UI style: {}", style);
        if style != "auto" {
            // Set the style and restart the app for it to take effect
            std::env::set_var("SLINT_STYLE", style.as_str());
            println!("UI style set to: {}. Please restart the application for changes to take effect.", style);
            // TODO: Show a toast notification about restart requirement
        } else {
            // Reset to auto-detected style
            std::env::remove_var("SLINT_STYLE");
            println!("UI style set to auto-detect. Please restart the application for changes to take effect.");
        }
    });
    
    // Initial refresh
    {
        let state = app_state.clone();
        slint::spawn_local(async move {
            if let Err(e) = state.refresh_vms().await {
                eprintln!("Failed to refresh VMs: {}", e);
            }
        }).unwrap();
    }
    
    // Set up periodic refresh
    {
        let state = app_state.clone();
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(1), move || {
            let state = state.clone();
            slint::spawn_local(async move {
                if let Err(e) = state.refresh_vms().await {
                    eprintln!("Failed to refresh VMs: {}", e);
                }
            }).unwrap();
        });
    }
    
    // Run the UI
    ui.run()?;
    
    Ok(())
}