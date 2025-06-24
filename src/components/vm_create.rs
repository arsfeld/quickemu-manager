use dioxus::prelude::*;
use crate::models::VMTemplate;

#[derive(Clone, PartialEq)]
pub struct OSInfo {
    pub name: String,
    pub versions: Vec<String>,
}

#[component]
pub fn VMCreate(
    on_create: EventHandler<VMTemplate>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut selected_os = use_signal(|| String::new());
    let mut selected_version = use_signal(|| String::new());
    let mut vm_name = use_signal(|| String::new());
    let mut ram = use_signal(|| "4G".to_string());
    let mut disk_size = use_signal(|| "64G".to_string());
    let mut cpu_cores = use_signal(|| 2u32);
    let mut creating = use_signal(|| false);
    let mut create_status = use_signal(|| String::new());
    let mut create_error = use_signal(|| Option::<String>::None);

    let supported_os = use_memo(|| {
        vec![
            OSInfo { name: "ubuntu".to_string(), versions: vec!["22.04".to_string(), "20.04".to_string(), "24.04".to_string()] },
            OSInfo { name: "fedora".to_string(), versions: vec!["39".to_string(), "38".to_string(), "40".to_string()] },
            OSInfo { name: "debian".to_string(), versions: vec!["12".to_string(), "11".to_string()] },
            OSInfo { name: "archlinux".to_string(), versions: vec!["latest".to_string()] },
            OSInfo { name: "manjaro".to_string(), versions: vec!["latest".to_string()] },
            OSInfo { name: "opensuse".to_string(), versions: vec!["tumbleweed".to_string(), "leap-15.5".to_string()] },
            OSInfo { name: "centos-stream".to_string(), versions: vec!["9".to_string(), "8".to_string()] },
            OSInfo { name: "windows".to_string(), versions: vec!["11".to_string(), "10".to_string()] },
        ]
    });

    let available_versions = use_memo(move || {
        let os = selected_os.read();
        if os.is_empty() {
            Vec::new()
        } else {
            supported_os.read()
                .iter()
                .find(|info| info.name == *os)
                .map(|info| info.versions.clone())
                .unwrap_or_default()
        }
    });

    let can_create = use_memo(move || {
        !vm_name.read().trim().is_empty() 
            && !selected_os.read().is_empty() 
            && !selected_version.read().is_empty()
            && !*creating.read()
    });

    rsx! {
        div { class: "vm-create-overlay",
            div { class: "vm-create-dialog",
                div { class: "vm-create-header",
                    h2 { "Create New Virtual Machine" }
                    button { 
                        class: "close-button",
                        onclick: move |_| on_cancel.call(()),
                        "×"  
                    }
                }
                
                div { class: "vm-create-content",
                    div { class: "form-group",
                        label { r#for: "vm-name", "VM Name" }
                        input {
                            id: "vm-name",
                            r#type: "text",
                            placeholder: "Enter VM name",
                            value: "{vm_name}",
                            oninput: move |e| vm_name.set(e.value()),
                            disabled: *creating.read()
                        }
                    }
                    
                    div { class: "form-row",
                        div { class: "form-group",
                            label { r#for: "os-select", "Operating System" }
                            select {
                                id: "os-select",
                                value: "{selected_os}",
                                onchange: move |e| {
                                    selected_os.set(e.value());
                                    selected_version.set(String::new());
                                },
                                disabled: *creating.read(),
                                option { value: "", "Select OS..." }
                                for os_info in supported_os.read().iter() {
                                    option { value: "{os_info.name}", "{os_info.name}" }
                                }
                            }
                        }
                        
                        div { class: "form-group",
                            label { r#for: "version-select", "Version" }
                            select {
                                id: "version-select",
                                value: "{selected_version}",
                                onchange: move |e| selected_version.set(e.value()),
                                disabled: *creating.read() || selected_os.read().is_empty(),
                                option { value: "", "Select version..." }
                                for version in available_versions.read().iter() {
                                    option { value: "{version}", "{version}" }
                                }
                            }
                        }
                    }
                    
                    div { class: "form-row",
                        div { class: "form-group",
                            label { r#for: "ram", "RAM" }
                            select {
                                id: "ram",
                                value: "{ram}",
                                onchange: move |e| ram.set(e.value()),
                                disabled: *creating.read(),
                                option { value: "2G", "2 GB" }
                                option { value: "4G", "4 GB" }
                                option { value: "8G", "8 GB" }
                                option { value: "16G", "16 GB" }
                                option { value: "32G", "32 GB" }
                            }
                        }
                        
                        div { class: "form-group",
                            label { r#for: "disk-size", "Disk Size" }
                            select {
                                id: "disk-size",
                                value: "{disk_size}",
                                onchange: move |e| disk_size.set(e.value()),
                                disabled: *creating.read(),
                                option { value: "32G", "32 GB" }
                                option { value: "64G", "64 GB" }
                                option { value: "128G", "128 GB" }
                                option { value: "256G", "256 GB" }
                                option { value: "512G", "512 GB" }
                            }
                        }
                        
                        div { class: "form-group",
                            label { r#for: "cpu-cores", "CPU Cores" }
                            select {
                                id: "cpu-cores",
                                value: "{cpu_cores}",
                                onchange: move |e| {
                                    if let Ok(cores) = e.value().parse::<u32>() {
                                        cpu_cores.set(cores);
                                    }
                                },
                                disabled: *creating.read(),
                                option { value: "1", "1 Core" }
                                option { value: "2", "2 Cores" }
                                option { value: "4", "4 Cores" }
                                option { value: "8", "8 Cores" }
                            }
                        }
                    }
                }
                
                // Progress and status display
                if *creating.read() {
                    div { class: "vm-create-progress",
                        div { class: "progress-spinner" }
                        div { class: "progress-text",
                            "{create_status.read()}"
                        }
                    }
                }
                
                // Error display
                if let Some(error) = create_error.read().as_ref() {
                    div { class: "vm-create-error",
                        div { class: "error-icon", "⚠️" }
                        div { class: "error-text", "{error}" }
                    }
                }
                
                div { class: "vm-create-actions",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_cancel.call(()),
                        disabled: *creating.read(),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: !*can_create.read(),
                        onclick: move |_| {
                            if *can_create.read() {
                                creating.set(true);
                                create_error.set(None);
                                create_status.set("Downloading and setting up VM files...".to_string());
                                
                                let template = VMTemplate {
                                    name: vm_name.read().trim().to_string(),
                                    os: selected_os.read().clone(),
                                    version: selected_version.read().clone(),
                                    ram: ram.read().clone(),
                                    disk_size: disk_size.read().clone(),
                                    cpu_cores: *cpu_cores.read(),
                                };
                                on_create.call(template);
                            }
                        },
                        if *creating.read() { 
                            "🔄 Creating VM..." 
                        } else { 
                            "✨ Create VM" 
                        }
                    }
                }
            }
        }
    }
}