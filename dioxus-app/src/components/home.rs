use dioxus::prelude::*;

use crate::models::{VM};
use crate::server_functions::{get_vms, get_vm_cache_version};
use crate::components::vm_card::VMCard;
use crate::components::vm_create_modal::CreateVMModal;
use crate::components::basic_console::BasicConsole;
use crate::components::settings_modal::SettingsModal;

/// Home page - VM Management Dashboard
#[component]
pub fn Home() -> Element {
    let mut vms = use_signal(Vec::<VM>::new);
    let mut show_create_modal = use_signal(|| false);
    let mut show_console = use_signal(|| None::<VM>);
    let mut show_settings = use_signal(|| false);
    let mut cache_version = use_signal(|| 0u64);
    
    // Function to refresh VM list
    let refresh_vms = move || {
        spawn(async move {
            if let Ok(vm_list) = get_vms().await {
                vms.set(vm_list);
            }
        });
    };
    
    // Load VMs on component mount and set up smart refresh
    use_effect(move || {
        // Initial load
        refresh_vms();
        
        // Set up background task to check for file system changes
        spawn(async move {
            loop {
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(2000).await;
                
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                
                // Check if cache version has changed
                if let Ok(new_version) = get_vm_cache_version().await {
                    let current_version = cache_version();
                    if new_version != current_version {
                        cache_version.set(new_version);
                        // Cache has changed, refresh VMs
                        if let Ok(vm_list) = get_vms().await {
                            vms.set(vm_list);
                        }
                    }
                }
            }
        });
    });
    
    rsx! {
        div { class: "container mx-auto px-4 py-8",
            // Header
            div { class: "flex items-center justify-between mb-8",
                div { class: "flex items-center space-x-4",
                    h1 { class: "text-3xl font-bold text-gray-800", "Virtual Machines" }
                }
                div { class: "flex items-center space-x-3",
                    button {
                        class: "btn-macos flex items-center gap-2",
                        onclick: move |_| show_settings.set(true),
                        svg { class: "w-4 h-4",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                            }
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                            }
                        }
                        "Settings"
                    }
                    
                    button {
                        class: "btn-macos-primary",
                        onclick: move |_| show_create_modal.set(true),
                        "Create VM"
                    }
                }
            }
            
            // VM Grid
            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-8",
                for vm in vms().iter() {
                    VMCard { 
                        vm: vm.clone(),
                        on_delete: move |deleted_id| {
                            // Remove the deleted VM from the list
                            vms.with_mut(|vm_list| {
                                vm_list.retain(|vm| vm.id != deleted_id);
                            });
                        },
                        on_status_change: move |_| {
                            // Refresh the VM list when status changes
                            refresh_vms();
                        },
                        on_card_click: move |vm| {
                            show_console.set(Some(vm));
                        }
                    }
                }
            }
            
            // Create VM Modal
            if show_create_modal() {
                CreateVMModal { 
                    show: show_create_modal,
                    on_create: move |_| {
                        // Refresh VM list after creation
                        refresh_vms();
                    }
                }
            }

            // Enhanced Console with eyeos SPICE client
            if let Some(vm) = show_console() {
                BasicConsole {
                    vm: vm,
                    on_close: move |_| show_console.set(None)
                }
            }

            // Settings Modal
            SettingsModal {
                show: show_settings
            }
        }
    }
}
