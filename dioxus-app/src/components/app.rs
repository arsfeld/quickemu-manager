use dioxus::prelude::*;
use crate::components::vm_card::VMCard;

#[cfg(feature = "web")]
use crate::api::{ApiClient, VM};

#[cfg(any(feature = "desktop", feature = "server"))]
use crate::services::{VM, VMManager};

#[component]
pub fn App() -> Element {
    let mut vms = use_signal(|| Vec::<VM>::new());
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    
    let load_vms = move || {
        spawn(async move {
            loading.set(true);
            error.set(None);
            
            #[cfg(feature = "web")]
            {
                let api = ApiClient::new();
                match api.list_vms().await {
                    Ok(vm_list) => {
                        vms.set(vm_list);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load VMs: {}", e)));
                        loading.set(false);
                    }
                }
            }
            
            #[cfg(any(feature = "desktop", feature = "server"))]
            {
                let manager = VMManager::new();
                match manager.list_vms().await {
                    Ok(vm_list) => {
                        vms.set(vm_list);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load VMs: {}", e)));
                        loading.set(false);
                    }
                }
            }
        });
    };

    // Load VMs on component mount
    use_effect(move || {
        load_vms();
    });

    // Set up periodic refresh
    use_effect(move || {
        let handle = spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                load_vms();
            }
        });
        
        move || {
            handle.cancel();
        }
    });

    rsx! {
        div { class: "container",
            div { class: "header",
                h1 { "Quickemu Manager" }
                button {
                    onclick: move |_| load_vms(),
                    disabled: *loading.read(),
                    "Refresh"
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "error",
                    "Error: " {err.clone()}
                }
            }

            if *loading.read() {
                div { class: "loading",
                    "Loading VMs..."
                }
            } else if vms.read().is_empty() {
                div { class: "loading",
                    "No VMs found."
                }
            } else {
                div { class: "vm-grid",
                    for vm in vms.read().iter() {
                        VMCard {
                            key: "{vm.id}",
                            vm: vm.clone(),
                            on_refresh: load_vms
                        }
                    }
                }
            }
        }
    }
}