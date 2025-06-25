use dioxus::prelude::*;
use crate::api::{ApiClient, VM, VMStatus};

#[derive(Props, Clone, PartialEq)]
pub struct VMCardProps {
    vm: VM,
    on_refresh: EventHandler<()>,
}

#[component]
pub fn VMCard(props: VMCardProps) -> Element {
    let mut is_loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let api_client = ApiClient::new();

    let vm = &props.vm;
    let status_class = match vm.status {
        VMStatus::Running => "status-running",
        VMStatus::Stopped => "status-stopped",
        VMStatus::Starting => "status-running",
        VMStatus::Stopping => "status-stopped",
    };

    let start_vm = {
        let vm_id = vm.id.clone();
        let api = api_client.clone();
        let on_refresh = props.on_refresh;
        move || {
            let vm_id = vm_id.clone();
            let api = api.clone();
            spawn(async move {
                is_loading.set(true);
                error.set(None);
                
                match api.start_vm(&vm_id).await {
                    Ok(_) => {
                        // Refresh the VM list after successful start
                        on_refresh(());
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to start VM: {}", e)));
                    }
                }
                is_loading.set(false);
            });
        }
    };

    let stop_vm = {
        let vm_id = vm.id.clone();
        let api = api_client.clone();
        let on_refresh = props.on_refresh;
        move || {
            let vm_id = vm_id.clone();
            let api = api.clone();
            spawn(async move {
                is_loading.set(true);
                error.set(None);
                
                match api.stop_vm(&vm_id).await {
                    Ok(_) => {
                        // Refresh the VM list after successful stop
                        on_refresh(());
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to stop VM: {}", e)));
                    }
                }
                is_loading.set(false);
            });
        }
    };

    rsx! {
        div { class: "vm-card",
            div { class: "vm-header",
                div {
                    h3 { {vm.name.clone()} }
                    p { {vm.os.clone()} " " {vm.version.clone()} }
                    p { "CPU: " {vm.cpu_cores.to_string()} " cores, RAM: " {vm.ram_mb.to_string()} " MB" }
                }
                span { class: "vm-status {status_class}", {vm.status.to_string()} }
            }
            
            if let Some(err) = error.read().as_ref() {
                div { class: "error",
                    {err.clone()}
                }
            }

            div { class: "vm-actions",
                {match vm.status {
                    VMStatus::Stopped => {
                        if *is_loading.read() {
                            rsx! {
                                button {
                                    class: "btn-start",
                                    disabled: true,
                                    "Starting..."
                                }
                            }
                        } else {
                            rsx! {
                                button {
                                    class: "btn-start",
                                    onclick: move |_| start_vm(),
                                    "Start"
                                }
                            }
                        }
                    }
                    VMStatus::Running => {
                        if *is_loading.read() {
                            rsx! {
                                button {
                                    class: "btn-stop",
                                    disabled: true,
                                    "Stopping..."
                                }
                            }
                        } else {
                            rsx! {
                                button {
                                    class: "btn-stop",
                                    onclick: move |_| stop_vm(),
                                    "Stop"
                                }
                            }
                        }
                    }
                    VMStatus::Starting => {
                        rsx! {
                            button {
                                disabled: true,
                                "Starting..."
                            }
                        }
                    }
                    VMStatus::Stopping => {
                        rsx! {
                            button {
                                disabled: true,
                                "Stopping..."
                            }
                        }
                    }
                }}
            }
        }
    }
}