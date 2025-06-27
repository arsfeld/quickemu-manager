use dioxus::prelude::*;

use crate::models::{VM, EditVMRequest};
use crate::server_functions::edit_vm;

/// VM Edit Modal Component
#[component]
pub fn VMEditModal(
    vm: VM,
    is_open: Signal<bool>,
    on_close: EventHandler<()>,
    on_update: EventHandler<()>,
) -> Element {
    // Store original values
    let vm_id = vm.id.clone();
    let vm_name_original = vm.name.clone();
    let vm_ram_mb_original = vm.ram_mb;
    let vm_cpu_cores_original = vm.cpu_cores;

    let mut vm_name = use_signal(|| vm_name_original.clone());
    let mut vm_ram_gb = use_signal(|| {
        // Convert MB to GB
        if vm_ram_mb_original >= 1024 {
            (vm_ram_mb_original / 1024) as i32
        } else {
            1 // Minimum 1GB
        }
    });
    let mut vm_cpu_cores = use_signal(|| vm_cpu_cores_original as i32);
    let mut is_saving = use_signal(|| false);
    let mut error_message = use_signal(|| String::new());

    // Reset form when modal opens
    {
        let vm_name_orig = vm_name_original.clone();
        let vm_ram_mb_orig = vm_ram_mb_original;
        let vm_cpu_cores_orig = vm_cpu_cores_original;
        use_effect(move || {
            if is_open() {
                vm_name.set(vm_name_orig.clone());
                vm_ram_gb.set(
                    if vm_ram_mb_orig >= 1024 {
                        (vm_ram_mb_orig / 1024) as i32
                    } else {
                        1 // Minimum 1GB
                    }
                );
                vm_cpu_cores.set(vm_cpu_cores_orig as i32);
                error_message.set(String::new());
            }
        });
    }

    let handle_save = move |_| {
        let vm_id = vm_id.clone();
        let name = vm_name().trim().to_string();
        let ram_gb = vm_ram_gb();
        let cpu_cores = vm_cpu_cores() as u32;
        let vm_name_orig = vm_name_original.clone();
        let vm_ram_mb_orig = vm_ram_mb_original;
        let vm_cpu_cores_orig = vm_cpu_cores_original;
        
        // Validate inputs
        if name.is_empty() {
            error_message.set("VM name cannot be empty".to_string());
            return;
        }
        
        if cpu_cores == 0 {
            error_message.set("CPU cores must be at least 1".to_string());
            return;
        }

        if ram_gb == 0 {
            error_message.set("RAM must be at least 1GB".to_string());
            return;
        }

        is_saving.set(true);
        error_message.set(String::new());
        
        spawn(async move {
            let request = EditVMRequest {
                vm_id,
                name: if name != vm_name_orig { Some(name) } else { None },
                ram: if ram_gb != (vm_ram_mb_orig / 1024) as i32 { 
                    Some(format!("{}G", ram_gb)) 
                } else { 
                    None 
                },
                cpu_cores: if cpu_cores != vm_cpu_cores_orig { Some(cpu_cores) } else { None },
            };
            
            match edit_vm(request).await {
                Ok(()) => {
                    is_saving.set(false);
                    on_update.call(());
                    on_close.call(());
                }
                Err(e) => {
                    is_saving.set(false);
                    error_message.set(e.to_string());
                }
            }
        });
    };

    if !is_open() {
        return rsx! { div {} };
    }

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |_| {
                if !is_saving() {
                    on_close.call(());
                }
            },
            
            div {
                class: "bg-white rounded-lg shadow-xl p-6 max-w-md mx-4 w-full",
                onclick: move |e| e.stop_propagation(),
                
                // Modal Header
                div { class: "flex items-center justify-between mb-6",
                    h3 { class: "text-xl font-semibold text-gray-900", "Edit Virtual Machine" }
                    button {
                        class: "text-gray-400 hover:text-gray-600",
                        onclick: move |_| {
                            if !is_saving() {
                                on_close.call(());
                            }
                        },
                        disabled: is_saving(),
                        "✕"
                    }
                }
                
                // Error Message
                if !error_message().is_empty() {
                    div { 
                        class: "mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded",
                        "{error_message()}"
                    }
                }
                
                // Form Fields
                div { class: "space-y-4",
                    
                    // VM Name
                    div {
                        label { 
                            class: "block text-sm font-medium text-gray-700 mb-1",
                            "VM Name"
                        }
                        input {
                            r#type: "text",
                            class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
                            value: "{vm_name()}",
                            disabled: is_saving(),
                            oninput: move |e| vm_name.set(e.value()),
                            placeholder: "Enter VM name"
                        }
                    }
                    
                    // RAM Configuration
                    div {
                        label { 
                            class: "block text-sm font-medium text-gray-700 mb-2", 
                            "RAM: {vm_ram_gb()} GB" 
                        }
                        input {
                            r#type: "range",
                            class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider",
                            min: "1",
                            max: "32",
                            value: "{vm_ram_gb()}",
                            disabled: is_saving(),
                            oninput: move |e| {
                                if let Ok(value) = e.value().parse::<i32>() {
                                    vm_ram_gb.set(value);
                                }
                            },
                        }
                        div { class: "flex justify-between text-xs text-gray-500 mt-1",
                            span { "1 GB" }
                            span { "32 GB" }
                        }
                    }
                    
                    // CPU Cores Configuration
                    div {
                        label { 
                            class: "block text-sm font-medium text-gray-700 mb-2", 
                            "CPU Cores: {vm_cpu_cores()}" 
                        }
                        input {
                            r#type: "range",
                            class: "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider",
                            min: "1",
                            max: "16",
                            value: "{vm_cpu_cores()}",
                            disabled: is_saving(),
                            oninput: move |e| {
                                if let Ok(value) = e.value().parse::<i32>() {
                                    vm_cpu_cores.set(value);
                                }
                            },
                        }
                        div { class: "flex justify-between text-xs text-gray-500 mt-1",
                            span { "1 core" }
                            span { "16 cores" }
                        }
                    }
                }
                
                // Action Buttons
                div { class: "flex justify-end space-x-3 mt-6",
                    button {
                        class: "btn-macos",
                        onclick: move |_| {
                            if !is_saving() {
                                on_close.call(());
                            }
                        },
                        disabled: is_saving(),
                        "Cancel"
                    }
                    button {
                        class: "btn-macos",
                        onclick: handle_save,
                        disabled: is_saving(),
                        if is_saving() { "Saving..." } else { "Save Changes" }
                    }
                }
            }
        }
    }
}