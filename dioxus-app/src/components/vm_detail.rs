use dioxus::prelude::*;

use crate::components::vm_console::VmConsole;
use crate::components::vm_edit_modal::VMEditModal;
use crate::components::vm_metrics::VMMetricsCard;
use crate::models::{VMStatus, VMStatusExt, VM};
use crate::server_functions::{delete_vm, start_vm, stop_vm, supports_console_access};

/// VM Detail Component - Desktop style
#[component]
pub fn VmDetail(
    vm: VM,
    on_close: EventHandler<()>,
    on_delete: EventHandler<String>,
    on_status_change: EventHandler<()>,
) -> Element {
    let vm_id_start = vm.id.clone();
    let vm_id_stop = vm.id.clone();
    let vm_id_delete = vm.id.clone();
    let vm_id_console = vm.id.clone();
    let vm_id_for_components = vm.id.clone();
    let is_running = vm.status.is_running();
    let mut show_delete_confirm = use_signal(|| false);
    let mut show_edit_modal = use_signal(|| false);
    let mut show_console = use_signal(|| false);
    let mut is_changing_state = use_signal(|| false);
    let mut console_supported = use_signal(|| false);

    // Check console support when VM is running
    use_effect(move || {
        if is_running {
            let console_id = vm_id_console.clone();
            spawn(async move {
                match supports_console_access(console_id).await {
                    Ok(supported) => console_supported.set(supported),
                    Err(_) => console_supported.set(false),
                }
            });
        } else {
            console_supported.set(false);
        }
    });

    rsx! {
        div { class: "flex flex-col h-full",
            // Header bar
            div { class: "h-12 px-4 flex items-center justify-between border-b border-macos-border bg-macos-surface",
                div { class: "flex items-center gap-3",
                    h2 { class: "text-sm font-medium text-macos-text", "{vm.name}" }
                    div { class: format!("px-2 py-0.5 rounded text-xs font-medium {}",
                        match &vm.status {
                            VMStatus::Running { .. } => "bg-macos-green-500 text-white",
                            VMStatus::Stopped => "bg-macos-gray-300 text-macos-gray-700",
                            VMStatus::Starting => "bg-macos-yellow-500 text-white",
                            VMStatus::Stopping => "bg-macos-orange-500 text-white",
                            VMStatus::Error(_) => "bg-macos-red-500 text-white",
                        }
                    ),
                        "{vm.status.display_text()}"
                    }
                }

                // Actions
                div { class: "flex items-center gap-2",
                    if is_running {
                        if console_supported() {
                            button {
                                class: "btn-macos h-7 px-3 text-xs",
                                onclick: move |_| show_console.set(!show_console()),
                                if show_console() { "Hide Console" } else { "Show Console" }
                            }
                        }
                        button {
                            class: "btn-macos h-7 px-3 text-xs",
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
                            class: "btn-macos-primary h-7 px-3 text-xs",
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
                        class: "btn-macos h-7 px-3 text-xs",
                        disabled: is_running,
                        onclick: move |_| show_edit_modal.set(true),
                        "Edit"
                    }

                    button {
                        class: "btn-macos h-7 px-3 text-xs",
                        onclick: move |_| show_delete_confirm.set(true),
                        "Delete"
                    }
                }
            }

            // Main content
            div { class: "flex-1 flex overflow-hidden",
                // Info panel
                div { class: format!("border-r border-macos-border bg-macos-surface overflow-y-auto {}",
                    if show_console() { "w-80" } else { "flex-1" }
                ),
                    div { class: "p-4 space-y-4",
                        // VM Info section
                        div {
                            h3 { class: "text-xs font-medium text-macos-text-secondary uppercase tracking-wider mb-3", "Configuration" }
                            div { class: "space-y-3",
                                // CPU
                                div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                    span { class: "text-sm text-macos-text-secondary", "CPU Cores" }
                                    if is_running {
                                        VMMetricsCard {
                                            vm_id: vm_id_for_components.clone(),
                                            vm_name: vm.name.clone(),
                                            metric_type: "cpu"
                                        }
                                    } else {
                                        span { class: "text-sm font-medium text-macos-text", "{vm.cpu_cores}" }
                                    }
                                }

                                // Memory
                                div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                    span { class: "text-sm text-macos-text-secondary", "Memory" }
                                    if is_running {
                                        VMMetricsCard {
                                            vm_id: vm_id_for_components.clone(),
                                            vm_name: vm.name.clone(),
                                            metric_type: "memory"
                                        }
                                    } else {
                                        span { class: "text-sm font-medium text-macos-text", "{vm.ram_mb / 1024} GB" }
                                    }
                                }

                                // Storage
                                div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                    span { class: "text-sm text-macos-text-secondary", "Storage" }
                                    span { class: "text-sm font-medium text-macos-text", "{vm.disk_size}" }
                                }

                                // OS
                                div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                    span { class: "text-sm text-macos-text-secondary", "Operating System" }
                                    span { class: "text-sm font-medium text-macos-text", "{vm.os}" }
                                }
                            }
                        }

                        // Network metrics if running
                        if is_running {
                            div {
                                h3 { class: "text-xs font-medium text-macos-text-secondary uppercase tracking-wider mb-3", "Network" }
                                div { class: "space-y-3",
                                    div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                        span { class: "text-sm text-macos-text-secondary", "Download" }
                                        VMMetricsCard {
                                            vm_id: vm_id_for_components.clone(),
                                            vm_name: vm.name.clone(),
                                            metric_type: "network_rx"
                                        }
                                    }
                                    div { class: "flex items-center justify-between py-2 border-b border-macos-border",
                                        span { class: "text-sm text-macos-text-secondary", "Upload" }
                                        VMMetricsCard {
                                            vm_id: vm_id_for_components.clone(),
                                            vm_name: vm.name.clone(),
                                            metric_type: "network_tx"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Console area
                if show_console() && is_running {
                    div { class: "flex-1 bg-black",
                        VmConsole {
                            vm: vm.clone(),
                            on_close: move |_| show_console.set(false),
                            inline_mode: true
                        }
                    }
                }
            }

            // Delete confirmation dialog
            if show_delete_confirm() {
                div {
                    class: "fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50",
                    onclick: move |_| show_delete_confirm.set(false),

                    div {
                        class: "bg-macos-surface rounded-macos-lg shadow-macos-xl p-6 max-w-sm mx-4",
                        onclick: move |e| e.stop_propagation(),

                        h3 { class: "text-base font-medium text-macos-text mb-2", "Delete \"{vm.name}\"?" }
                        p { class: "text-sm text-macos-text-secondary mb-4",
                            "This action cannot be undone. All VM data will be permanently deleted."
                        }

                        div { class: "flex justify-end gap-2",
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
                                "Delete VM"
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
