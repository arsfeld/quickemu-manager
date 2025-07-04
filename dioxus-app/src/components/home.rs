use dioxus::prelude::*;

use crate::components::keyboard_shortcuts::KeyboardShortcuts;
use crate::components::settings_modal::SettingsModal;
use crate::components::vm_console::VmConsole;
use crate::components::vm_create_modal::CreateVMModal;
use crate::components::vm_detail::VmDetail;
use crate::models::{VMStatus, VMStatusExt, VM};
use crate::server_functions::{get_vm_cache_version, get_vms};

/// Home page - VM Management Dashboard with desktop layout
#[component]
pub fn Home() -> Element {
    let mut vms = use_signal(Vec::<VM>::new);
    let mut show_create_modal = use_signal(|| false);
    let mut selected_vm = use_signal(|| None::<VM>);
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
        // Keyboard shortcuts handler
        KeyboardShortcuts {
            on_new_vm: move |_| show_create_modal.set(true),
            on_settings: move |_| show_settings.set(true),
            on_refresh: move |_| refresh_vms()
        }

        div { class: "h-screen flex bg-macos-background",
            // Sidebar
            div { class: "w-64 bg-macos-gray-50 border-r border-macos-border flex flex-col",
                // Sidebar header
                div { class: "h-12 px-4 flex items-center justify-between border-b border-macos-border",
                    span { class: "text-xs font-medium text-macos-text-secondary uppercase tracking-wider", "Virtual Machines" }
                    button {
                        class: "text-macos-blue-500 hover:text-macos-blue-600 transition-colors",
                        onclick: move |_| show_create_modal.set(true),
                        title: "Create new VM",
                        svg { class: "w-4 h-4",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M12 4v16m8-8H4"
                            }
                        }
                    }
                }

                // VM list
                div { class: "flex-1 overflow-y-auto native-scrollbar",
                    if vms().is_empty() {
                        div { class: "p-4 text-center text-sm text-macos-text-tertiary",
                            p { "No virtual machines" }
                            button {
                                class: "mt-2 text-macos-blue-500 hover:text-macos-blue-600",
                                onclick: move |_| show_create_modal.set(true),
                                "Create your first VM"
                            }
                        }
                    } else {
                        div { class: "py-1",
                            {vms().clone().into_iter().map(|vm| {
                                let vm_for_click = vm.clone();
                                rsx! {
                                    button {
                                    class: format!(
                                        "w-full px-3 py-2 flex items-center gap-2 hover:bg-macos-gray-100 transition-colors text-left group {}",
                                        if selected_vm().as_ref().map(|v| v.id == vm.id).unwrap_or(false) {
                                            "bg-macos-blue-500 text-white hover:bg-macos-blue-600"
                                        } else {
                                            ""
                                        }
                                    ),
                                    onclick: move |_| {
                                        selected_vm.set(Some(vm_for_click.clone()));
                                    },

                                    // Status indicator
                                    div {
                                        class: format!("w-2 h-2 rounded-full flex-shrink-0 {}",
                                            match &vm.status {
                                                VMStatus::Running { .. } => "bg-macos-green-500",
                                                VMStatus::Stopped => "bg-macos-gray-400",
                                                VMStatus::Starting => "bg-macos-yellow-500",
                                                VMStatus::Stopping => "bg-macos-orange-500",
                                                VMStatus::Error(_) => "bg-macos-red-500",
                                            }
                                        )
                                    }

                                    // VM name and info
                                    div { class: "flex-1 min-w-0",
                                        div { class: "text-sm font-medium truncate", "{vm.name}" }
                                        div {
                                            class: format!("text-xs truncate {}",
                                                if selected_vm().as_ref().map(|v| v.id == vm.id).unwrap_or(false) {
                                                    "text-white/70"
                                                } else {
                                                    "text-macos-text-tertiary"
                                                }
                                            ),
                                            "{vm.os}"
                                        }
                                    }
                                }
                                }
                            })}
                        }
                    }
                }

                // Sidebar footer
                div { class: "border-t border-macos-border p-2",
                    button {
                        class: "w-full btn-macos text-xs flex items-center justify-center gap-2",
                        onclick: move |_| show_settings.set(true),
                        svg { class: "w-3.5 h-3.5",
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
                        "Preferences"
                    }
                }
            }

            // Main content area
            div { class: "flex-1 flex flex-col overflow-hidden",
                if let Some(vm) = selected_vm() {
                    VmDetail {
                        vm: vm.clone(),
                        on_close: move |_| selected_vm.set(None),
                        on_delete: move |deleted_id| {
                            vms.with_mut(|vm_list| {
                                vm_list.retain(|vm| vm.id != deleted_id);
                            });
                            selected_vm.set(None);
                        },
                        on_status_change: move |_| {
                            refresh_vms();
                        }
                    }
                } else {
                    // Empty state
                    div { class: "flex-1 flex items-center justify-center",
                        div { class: "text-center max-w-md",
                            svg { class: "w-20 h-20 mx-auto mb-4 text-macos-gray-300",
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "1",
                                    d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                                }
                            }
                            h3 { class: "text-lg font-medium text-macos-text mb-2", "No VM Selected" }
                            p { class: "text-sm text-macos-text-secondary mb-4",
                                "Select a virtual machine from the sidebar to view details and manage it"
                            }
                            if vms().is_empty() {
                                button {
                                    class: "btn-macos-primary",
                                    onclick: move |_| show_create_modal.set(true),
                                    "Create Virtual Machine"
                                }
                            }
                        }
                    }
                }
            }

            // Create VM Modal
            if show_create_modal() {
                CreateVMModal {
                    show: show_create_modal,
                    on_create: move |_| {
                        refresh_vms();
                    }
                }
            }

            // Settings Modal
            SettingsModal {
                show: show_settings
            }
        }
    }
}
