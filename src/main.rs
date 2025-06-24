mod models;
mod services;
mod components;

use dioxus::prelude::*;
use models::{AppConfig, VM, VMId, VMTemplate};
use services::{VMDiscovery, VMManager, ProcessMonitor, MetricsService, DiscoveryEvent, QuickgetService};
use components::{Header, VMList, VMDetail, Settings, VMCreate};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<AppConfig>>,
    vm_manager: Arc<VMManager>,
    process_monitor: Arc<ProcessMonitor>,
    metrics_service: Arc<MetricsService>,
    quickget_service: Option<Arc<QuickgetService>>,
    vms: Arc<RwLock<Vec<VM>>>,
}

#[derive(Clone, PartialEq)]
enum View {
    VMList,
    VMDetail(VMId),
    Settings,
    CreateVM,
}

fn main() {
    dioxus::launch(app);
}

#[component]
fn App() -> Element {
    let mut current_view = use_signal(|| View::VMList);
    let app_state = use_resource(|| async {
        let config = AppConfig::load().unwrap_or_default();
        let vm_manager = VMManager::new().unwrap_or_else(|_| {
            VMManager::with_paths(
                std::path::PathBuf::from("quickemu"),
                None
            )
        });
        
        let quickget_service = if vm_manager.is_quickget_available() {
            which::which("quickget")
                .ok()
                .map(|path| Arc::new(QuickgetService::new(path)))
        } else {
            None
        };
        
        AppState {
            config: Arc::new(RwLock::new(config)),
            vm_manager: Arc::new(vm_manager),
            process_monitor: Arc::new(ProcessMonitor::new()),
            metrics_service: Arc::new(MetricsService::new(60)),
            quickget_service,
            vms: Arc::new(RwLock::new(Vec::new())),
        }
    });
    
    let vms = use_signal(|| Vec::<VM>::new());
    
    // Initialize VM discovery
    use_effect(move || {
        if let Some(state) = app_state.read().as_ref() {
            let state = state.clone();
            let mut vms = vms.clone();
            
            spawn(async move {
                let (event_tx, mut event_rx) = mpsc::unbounded_channel();
                let mut discovery = VMDiscovery::new(event_tx);
                
                // Scan configured directories
                let config = state.config.read().await;
                for dir in &config.vm_directories {
                    if let Ok(discovered_vms) = discovery.scan_directory(dir).await {
                        let mut all_vms = state.vms.write().await;
                        for vm in discovered_vms {
                            all_vms.push(vm);
                        }
                        vms.set(all_vms.clone());
                    }
                }
                
                // Start watching for changes
                let _ = discovery.start_watching().await;
                
                // Handle discovery events
                while let Some(event) = event_rx.recv().await {
                    match event {
                        DiscoveryEvent::VMAdded(vm) => {
                            state.vms.write().await.push(vm);
                            vms.set(state.vms.read().await.clone());
                        }
                        DiscoveryEvent::VMUpdated(updated_vm) => {
                            let mut vm_list = state.vms.write().await;
                            if let Some(vm) = vm_list.iter_mut().find(|v| v.id == updated_vm.id) {
                                *vm = updated_vm;
                            }
                            vms.set(vm_list.clone());
                        }
                        DiscoveryEvent::VMRemoved(id) => {
                            state.vms.write().await.retain(|v| v.id != id);
                            vms.set(state.vms.read().await.clone());
                        }
                    }
                }
            });
        }
    });
    
    // Periodic metrics update
    use_effect(move || {
        if let Some(state) = app_state.read().as_ref() {
            let state = state.clone();
            spawn(async move {
                loop {
                    state.process_monitor.update_metrics().await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            });
        }
    });
    
    rsx! {
        style { {include_str!("modern.css")} }
        
        div { class: "app",
            Header {
                on_create_vm: move |_| {
                    current_view.set(View::CreateVM);
                },
                on_settings: move |_| {
                    current_view.set(View::Settings);
                },
                on_refresh: move |_| {
                    if let Some(state) = app_state.read().as_ref() {
                        let state = state.clone();
                        let mut vms = vms.clone();
                        spawn(async move {
                            let mut vm_list = state.vms.write().await;
                            for vm in vm_list.iter_mut() {
                                state.vm_manager.update_vm_status(vm).await;
                            }
                            vms.set(vm_list.clone());
                        });
                    }
                }
            }
            
            div { class: "app-content",
                match current_view.read().clone() {
                    View::VMList => {
                        let vms_list = vms.read().clone();
                        rsx! {
                            VMList {
                                vms: vms_list,
                                on_vm_start: move |id| {
                                    if let Some(state) = app_state.read().as_ref() {
                                        let state = state.clone();
                                        spawn(async move {
                                            let vm_list = state.vms.read().await;
                                            if let Some(vm) = vm_list.iter().find(|v| v.id == id) {
                                                if let Err(e) = state.vm_manager.start_vm(vm).await {
                                                    eprintln!("Failed to start VM: {}", e);
                                                }
                                            }
                                        });
                                    }
                                },
                                on_vm_stop: move |id| {
                                    if let Some(state) = app_state.read().as_ref() {
                                        let state = state.clone();
                                        spawn(async move {
                                            if let Err(e) = state.vm_manager.stop_vm(&id).await {
                                                eprintln!("Failed to stop VM: {}", e);
                                            }
                                        });
                                    }
                                },
                                on_vm_select: move |id| {
                                    current_view.set(View::VMDetail(id));
                                }
                            }
                        }
                    },
                    View::VMDetail(vm_id) => {
                        let vms_list = vms.read().clone();
                        if let Some(vm) = vms_list.iter().find(|v| v.id == vm_id) {
                            let vm_clone = vm.clone();
                            let vm_clone_start = vm_clone.clone();
                            let vm_clone_stop = vm_clone.clone();
                            let vm_clone_restart = vm_clone.clone();
                            let vm_clone_console = vm_clone.clone();
                            rsx! {
                                VMDetail {
                                    vm: vm_clone.clone(),
                                    metrics: None,
                                    history: None,
                                    on_start: move |_| {
                                        if let Some(state) = app_state.read().as_ref() {
                                            let state = state.clone();
                                            let vm = vm_clone_start.clone();
                                            spawn(async move {
                                                if let Err(e) = state.vm_manager.start_vm(&vm).await {
                                                    eprintln!("Failed to start VM: {}", e);
                                                }
                                            });
                                        }
                                    },
                                    on_stop: move |_| {
                                        if let Some(state) = app_state.read().as_ref() {
                                            let state = state.clone();
                                            let id = vm_clone_stop.id.clone();
                                            spawn(async move {
                                                if let Err(e) = state.vm_manager.stop_vm(&id).await {
                                                    eprintln!("Failed to stop VM: {}", e);
                                                }
                                            });
                                        }
                                    },
                                    on_restart: move |_| {
                                        if let Some(state) = app_state.read().as_ref() {
                                            let state = state.clone();
                                            let vm = vm_clone_restart.clone();
                                            spawn(async move {
                                                if let Err(e) = state.vm_manager.restart_vm(&vm).await {
                                                    eprintln!("Failed to restart VM: {}", e);
                                                }
                                            });
                                        }
                                    },
                                    on_console: move |_| {
                                        if let Some(state) = app_state.read().as_ref() {
                                            let state = state.clone();
                                            let vm = vm_clone_console.clone();
                                            spawn(async move {
                                                if let Err(e) = state.vm_manager.launch_display(&vm).await {
                                                    eprintln!("Failed to launch console: {}", e);
                                                }
                                            });
                                        }
                                    },
                                    on_close: move |_| {
                                        current_view.set(View::VMList);
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { "VM not found" }
                            }
                        }
                    },
                    View::CreateVM => {
                        if let Some(state) = app_state.read().as_ref() {
                            if state.quickget_service.is_some() {
                                let state = state.clone();
                                let mut vms = vms.clone();
                                rsx! {
                                    VMCreate {
                                        on_create: move |template: VMTemplate| {
                                            let state = state.clone();
                                            let mut vms = vms.clone();
                                            spawn(async move {
                                                let config = state.config.read().await;
                                                let target_dir = config.vm_directories
                                                    .first()
                                                    .cloned()
                                                    .unwrap_or_else(|| std::path::PathBuf::from("./vms"))
                                                    .join(&template.name);
                                                
                                                // Create VM with quickget
                                                match state.vm_manager.create_vm(template.clone(), &target_dir).await {
                                                    Ok(config_path) => {
                                                        println!("VM created successfully: {}", config_path.display());
                                                        
                                                        // Wait a moment for files to be written
                                                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                                        
                                                        // Trigger VM discovery refresh
                                                        let (event_tx, _) = mpsc::unbounded_channel();
                                                        let mut discovery = VMDiscovery::new(event_tx);
                                                        if let Ok(new_vms) = discovery.scan_directory(&target_dir.parent().unwrap_or(&target_dir)).await {
                                                            for new_vm in new_vms {
                                                                // Check if VM already exists to avoid duplicates
                                                                let mut vm_list = state.vms.write().await;
                                                                if !vm_list.iter().any(|v| v.id == new_vm.id) {
                                                                    vm_list.push(new_vm);
                                                                }
                                                            }
                                                            vms.set(state.vms.read().await.clone());
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to create VM: {}", e);
                                                        // Note: In a real app, we'd want to show this error in the UI
                                                        // For now, it will appear in the console
                                                    }
                                                }
                                            });
                                            current_view.set(View::VMList);
                                        },
                                        on_cancel: move |_| {
                                            current_view.set(View::VMList);
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "error-message",
                                        h2 { "Quickget Not Available" }
                                        p { "VM creation requires quickget to be installed and available in your PATH." }
                                        button {
                                            onclick: move |_| current_view.set(View::VMList),
                                            "Back to VM List"
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! { div { "Loading..." } }
                        }
                    },
                    View::Settings => {
                        match app_state.read().as_ref() {
                            Some(state) => {
                                let state = state.clone();
                                let config_resource = use_resource(move || {
                                    let state = state.clone();
                                    async move {
                                        state.config.read().await.clone()
                                    }
                                });
                                
                                let config_value = config_resource.read();
                                match config_value.as_ref() {
                                    Some(config_data) => {
                                        let config_clone = config_data.clone();
                                        rsx! {
                                            Settings {
                                                config: config_clone,
                                                on_save: move |new_config: AppConfig| {
                                                    if let Some(state) = app_state.read().as_ref() {
                                                        let state = state.clone();
                                                        spawn(async move {
                                                            *state.config.write().await = new_config.clone();
                                                            if let Err(e) = new_config.save() {
                                                                eprintln!("Failed to save config: {}", e);
                                                            }
                                                        });
                                                    }
                                                    current_view.set(View::VMList);
                                                },
                                                on_close: move |_| {
                                                    current_view.set(View::VMList);
                                                }
                                            }
                                        }
                                    },
                                    None => rsx! { div { "Loading..." } }
                                }
                            },
                            None => rsx! { div { "Loading..." } }
                        }
                    }
                }
            }
        }
    }
}

// Create the app function that was expected by the old launch call
fn app() -> Element {
    rsx! { App {} }
}