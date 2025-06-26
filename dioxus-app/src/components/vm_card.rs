use dioxus::prelude::*;

use crate::models::{VM, VMStatus, VMStatusExt};
use crate::server_functions::{start_vm, stop_vm, delete_vm};

/// VM Card Component
#[component]
pub fn VMCard(
    vm: VM, 
    on_delete: EventHandler<String>, 
    on_status_change: EventHandler<()>,
    on_card_click: EventHandler<VM>,
) -> Element {
    let vm_id_start = vm.id.clone();
    let vm_id_stop = vm.id.clone();
    let vm_id_delete = vm.id.clone();
    let is_running = vm.status.is_running();
    let mut show_delete_confirm = use_signal(|| false);
    let mut is_changing_state = use_signal(|| false);
    
    let card_class = if is_running {
        "card-macos hover:shadow-lg transition-shadow duration-200 cursor-pointer"
    } else {
        "card-macos hover:shadow-lg transition-shadow duration-200"
    };

    let vm_clone_for_click = vm.clone();
    
    rsx! {
        div { 
            class: "{card_class}",
            onclick: move |_| {
                if is_running {
                    on_card_click.call(vm_clone_for_click.clone());
                }
            },

            // VM Header
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-xl font-semibold text-gray-800", "{vm.name}" }
                div { 
                    class: format!("px-3 py-1 rounded-full text-sm font-medium {}",
                        match &vm.status {
                            VMStatus::Running { .. } => "bg-green-100 text-green-800",
                            VMStatus::Stopped => "bg-gray-100 text-gray-800",
                            VMStatus::Starting => "bg-yellow-100 text-yellow-800",
                            VMStatus::Stopping => "bg-orange-100 text-orange-800",
                            VMStatus::Error(_) => "bg-red-100 text-red-800",
                        }
                    ),
                    
                    "{vm.status.display_text()}"
                }
            }
            
            // VM Info
            div { class: "space-y-2 mb-4",
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "OS:" }
                    span { class: "text-gray-800", "{vm.os} {vm.version}" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "CPU:" }
                    span { class: "text-gray-800", "{vm.cpu_cores} cores" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "RAM:" }
                    span { class: "text-gray-800", "{vm.ram_mb / 1024}GB" }
                }
                div { class: "flex justify-between text-sm",
                    span { class: "text-gray-600 font-medium", "Disk:" }
                    span { class: "text-gray-800", "{vm.disk_size}" }
                }
            }
            
            // VM Actions
            div { 
                class: "flex justify-end space-x-2",
                onclick: move |e| e.stop_propagation(),
                if is_running {
                    button {
                        class: "btn-macos-destructive",
                        disabled: is_changing_state(),
                        onclick: move |_| {
                            let id = vm_id_stop.clone();
                            is_changing_state.set(true);
                            spawn(async move {
                                let _ = stop_vm(id).await;
                                
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::TimeoutFuture::new(500).await;
                                
                                #[cfg(not(target_arch = "wasm32"))]
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                is_changing_state.set(false);
                                on_status_change.call(());
                            });
                        },
                        if is_changing_state() { "Stopping..." } else { "Stop" }
                    }
                } else {
                    button {
                        class: "btn-macos",
                        disabled: is_changing_state(),
                        onclick: move |_| {
                            let id = vm_id_start.clone();
                            is_changing_state.set(true);
                            spawn(async move {
                                let _ = start_vm(id).await;
                                
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::TimeoutFuture::new(500).await;
                                
                                #[cfg(not(target_arch = "wasm32"))]
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                is_changing_state.set(false);
                                on_status_change.call(());
                            });
                        },
                        if is_changing_state() { "Starting..." } else { "Start" }
                    }
                }
                
                button {
                    class: "btn-macos",
                    onclick: move |_| show_delete_confirm.set(true),
                    "Delete"
                }
            }
            
            // Delete confirmation modal
            if show_delete_confirm() {
                div { 
                    class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                    onclick: move |_| show_delete_confirm.set(false),
                    
                    div { 
                        class: "bg-white rounded-lg shadow-xl p-6 max-w-md mx-4",
                        onclick: move |e| e.stop_propagation(),
                        
                        h3 { class: "text-lg font-semibold text-gray-900 mb-4", "Delete Virtual Machine" }
                        p { class: "text-gray-600 mb-6", 
                            "Are you sure you want to delete \"{vm.name}\"? This action cannot be undone and will permanently remove all VM files."
                        }
                        
                        div { class: "flex justify-end space-x-3",
                            button {
                                class: "btn-macos",
                                onclick: move |_| show_delete_confirm.set(false),
                                "Cancel"
                            }
                            button {
                                class: "btn-macos-destructive",
                                onclick: move |_| {
                                    let id = vm_id_delete.clone();
                                    show_delete_confirm.set(false);
                                    spawn(async move {
                                        if let Ok(()) = delete_vm(id.clone()).await {
                                            on_delete.call(id);
                                        }
                                    });
                                },
                                "Delete"
                            }
                        }
                    }
                }
            }
        }
    }
}