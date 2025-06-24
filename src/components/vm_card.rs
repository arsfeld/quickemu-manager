use dioxus::prelude::*;
use crate::models::{VM, VMStatus, VMId};

#[component]
pub fn VMCard(
    vm: VM,
    on_start: EventHandler<VMId>,
    on_stop: EventHandler<VMId>,
    on_click: EventHandler<VMId>,
) -> Element {
    let vm_id = vm.id.clone();
    let vm_id_start = vm.id.clone();
    let vm_id_stop = vm.id.clone();
    
    let status_class = match &vm.status {
        VMStatus::Running { .. } => "vm-status running",
        VMStatus::Stopped => "vm-status stopped",
        VMStatus::Starting => "vm-status running",
        VMStatus::Stopping => "vm-status stopped",
        VMStatus::Error(_) => "vm-status error",
    };
    
    let status_dot_class = match &vm.status {
        VMStatus::Running { .. } => "status-dot running",
        VMStatus::Stopped => "status-dot stopped", 
        VMStatus::Error(_) => "status-dot error",
        _ => "status-dot stopped"
    };
    
    let status_text = match &vm.status {
        VMStatus::Running { .. } => "Running",
        VMStatus::Stopped => "Stopped",
        VMStatus::Starting => "Starting",
        VMStatus::Stopping => "Stopping",
        VMStatus::Error(_) => "Error",
    };
    
    rsx! {
        div {
            class: "vm-card",
            onclick: move |_| on_click(vm_id.clone()),
            
            div { class: "vm-card-header",
                div {
                    h3 { class: "vm-name", "{vm.name}" }
                    div { class: "vm-info-item",
                        div { class: "vm-info-label", "Guest OS" }
                        div { class: "vm-info-value", "{vm.config.guest_os}" }
                    }
                }
                div { 
                    class: "{status_class}",
                    div { class: "{status_dot_class}" }
                    "{status_text}"
                }
            }
            
            div { class: "vm-card-body",
                div { class: "vm-info",
                    div { class: "vm-info-item",
                        div { class: "vm-info-label", "CPU Cores" }
                        div { class: "vm-info-value", "{vm.config.cpu_cores}" }
                    }
                    div { class: "vm-info-item",
                        div { class: "vm-info-label", "Memory" }
                        div { class: "vm-info-value", "{vm.config.ram}" }
                    }
                }
            }
            
            div { class: "vm-card-actions",
                match &vm.status {
                    VMStatus::Stopped => rsx! {
                        button {
                            class: "btn btn-primary",
                            onclick: move |e| {
                                e.stop_propagation();
                                on_start(vm_id_start.clone())
                            },
                            "▶ Start"
                        }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |e| {
                                e.stop_propagation();
                                // TODO: Open VM details/settings
                            },
                            "⚙ Settings"
                        }
                    },
                    VMStatus::Running { .. } => rsx! {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |e| {
                                e.stop_propagation();
                                on_stop(vm_id_stop.clone())
                            },
                            "⏹ Stop"
                        }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |e| {
                                e.stop_propagation();
                                // TODO: Open console
                            },
                            "🖥 Console"
                        }
                    },
                    _ => rsx! {
                        button {
                            class: "btn btn-secondary",
                            disabled: true,
                            "{status_text}..."
                        }
                    }
                }
            }
        }
    }
}