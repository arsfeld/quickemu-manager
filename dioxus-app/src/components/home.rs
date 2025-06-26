use dioxus::prelude::*;

use crate::models::{VM};
use crate::server_functions::{get_vms};
use crate::components::vm_card::VMCard;
use crate::components::vm_create_modal::CreateVMModal;
use crate::components::vm_screen_modal::VMScreenModal;

/// Home page - VM Management Dashboard
#[component]
pub fn Home() -> Element {
    let mut vms = use_signal(Vec::<VM>::new);
    let mut show_create_modal = use_signal(|| false);
    let mut show_screen_modal = use_signal(|| None::<VM>);
    let mut last_updated = use_signal(|| None::<String>);
    
    // Function to refresh VM list
    let refresh_vms = move || {
        spawn(async move {
            if let Ok(vm_list) = get_vms().await {
                vms.set(vm_list);
                last_updated.set(Some("Updated".to_string()));
            }
        });
    };
    
    // Load VMs on component mount and set up periodic refresh
    use_effect(move || {
        // Initial load
        refresh_vms();
        
        // Set up periodic refresh every 3 seconds
        spawn(async move {
            loop {
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(3000).await;
                
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
                
                // Refresh VM list
                if let Ok(vm_list) = get_vms().await {
                    vms.set(vm_list);
                    last_updated.set(Some("Updated".to_string()));
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
                    if let Some(_) = last_updated() {
                        div { 
                            class: "flex items-center space-x-2 text-sm text-gray-500",
                            div { class: "w-2 h-2 bg-green-500 rounded-full animate-pulse" }
                            span { "Auto-refresh: 3s" }
                        }
                    }
                }
                button {
                    class: "btn-macos-primary",
                    onclick: move |_| show_create_modal.set(true),
                    "Create VM"
                }
            }
            
            // VM Grid
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
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
                            show_screen_modal.set(Some(vm));
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

            // VM Screen Modal
            if let Some(vm) = show_screen_modal() {
                VMScreenModal {
                    vm: vm,
                    on_close: move |_| show_screen_modal.set(None)
                }
            }
        }
    }
}
