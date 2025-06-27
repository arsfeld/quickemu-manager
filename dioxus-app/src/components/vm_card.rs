use dioxus::prelude::*;

use crate::models::{VM, VMStatus, VMStatusExt};
use crate::server_functions::{start_vm, stop_vm, delete_vm};
use crate::components::vm_edit_modal::VMEditModal;
use crate::components::vm_metrics::VMMetricsCard;

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
    let mut show_edit_modal = use_signal(|| false);
    let mut is_changing_state = use_signal(|| false);
    
    let card_class = if is_running {
        "bg-slate-900/40 backdrop-blur-xl border border-slate-700/50 rounded-2xl p-4 hover:shadow-xl hover:border-slate-600/60 transition-all duration-300 cursor-pointer shadow-lg flex flex-col w-full max-w-2xl"
    } else {
        "bg-slate-900/40 backdrop-blur-xl border border-slate-700/50 rounded-2xl p-4 hover:shadow-xl hover:border-slate-600/60 transition-all duration-300 shadow-lg flex flex-col w-full max-w-2xl"
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
                h3 { class: "text-xl font-semibold text-slate-100", "{vm.name}" }
                div { 
                    class: format!("px-3 py-1 rounded-full text-sm font-medium {}",
                        match &vm.status {
                            VMStatus::Running { .. } => "bg-green-500/30 text-green-200 border border-green-400/50",
                            VMStatus::Stopped => "bg-gray-500/30 text-gray-200 border border-gray-400/50",
                            VMStatus::Starting => "bg-yellow-500/30 text-yellow-200 border border-yellow-400/50",
                            VMStatus::Stopping => "bg-orange-500/30 text-orange-200 border border-orange-400/50",
                            VMStatus::Error(_) => "bg-red-500/30 text-red-200 border border-red-400/50",
                        }
                    ),
                    
                    "{vm.status.display_text()}"
                }
            }
            
            // VM Info Blocks - Compact but readable layout
            div { class: "flex space-x-2 mb-3",
                
                // CPU Block
                div { class: "flex-1 flex flex-col items-center justify-center h-16 px-3 bg-slate-800/30 rounded-xl border border-slate-600/40 backdrop-blur-sm hover:bg-slate-700/40 transition-all duration-200",
                    if is_running {
                        VMMetricsCard {
                            vm_id: vm.id.clone(),
                            vm_name: vm.name.clone(),
                            metric_type: "cpu"
                        }
                    } else {
                        div { class: "text-center",
                            div { class: "text-lg font-bold text-slate-100 mb-0.5", "{vm.cpu_cores}" }
                            span { class: "text-xs text-slate-400 uppercase tracking-wide", "CPU cores" }
                        }
                    }
                }
                
                // RAM Block  
                div { class: "flex-1 flex flex-col items-center justify-center h-16 px-3 bg-slate-800/30 rounded-xl border border-slate-600/40 backdrop-blur-sm hover:bg-slate-700/40 transition-all duration-200",
                    if is_running {
                        VMMetricsCard {
                            vm_id: vm.id.clone(),
                            vm_name: vm.name.clone(),
                            metric_type: "memory"
                        }
                    } else {
                        div { class: "text-center",
                            div { class: "text-lg font-bold text-slate-100 mb-0.5", "{vm.ram_mb / 1024}GB" }
                            span { class: "text-xs text-slate-400 uppercase tracking-wide", "Memory" }
                        }
                    }
                }
                
                // OS Block
                div { class: "flex-1 flex flex-col items-center justify-center h-16 px-3 bg-slate-800/30 rounded-xl border border-slate-600/40 backdrop-blur-sm hover:bg-slate-700/40 transition-all duration-200",
                    if is_running {
                        VMMetricsCard {
                            vm_id: vm.id.clone(),
                            vm_name: vm.name.clone(),
                            metric_type: "network_rx"
                        }
                    } else {
                        div { class: "text-center",
                            div { class: "text-sm font-bold text-slate-100 mb-0.5 truncate", "{vm.os}" }
                            span { class: "text-xs text-slate-400 uppercase tracking-wide", "OS" }
                        }
                    }
                }
                
                // Storage Block
                div { class: "flex-1 flex flex-col items-center justify-center h-16 px-3 bg-slate-800/30 rounded-xl border border-slate-600/40 backdrop-blur-sm hover:bg-slate-700/40 transition-all duration-200",
                    if is_running {
                        VMMetricsCard {
                            vm_id: vm.id.clone(),
                            vm_name: vm.name.clone(),
                            metric_type: "network_tx"
                        }
                    } else {
                        div { class: "text-center",
                            div { class: "text-lg font-bold text-slate-100 mb-0.5", "{vm.disk_size}" }
                            span { class: "text-xs text-slate-400 uppercase tracking-wide", "Storage" }
                        }
                    }
                }
            }
            
            // VM Actions
            div { 
                class: "flex justify-end space-x-2 mt-2",
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
                    disabled: is_running,
                    onclick: move |_| show_edit_modal.set(true),
                    "Edit"
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
                        class: "modal-macos p-6 max-w-md mx-4",
                        onclick: move |e| e.stop_propagation(),
                        
                        h3 { class: "text-lg font-semibold text-white mb-4", "Delete Virtual Machine" }
                        p { class: "text-gray-300 mb-6", 
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
            
            // Edit VM modal
            VMEditModal {
                vm: vm.clone(),
                is_open: show_edit_modal,
                on_close: move |_| show_edit_modal.set(false),
                on_update: move |_| {
                    show_edit_modal.set(false);
                    on_status_change.call(());
                }
            }
        }
    }
}