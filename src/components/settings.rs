use dioxus::prelude::*;
use crate::models::AppConfig;

#[component]
pub fn Settings(
    config: AppConfig,
    on_save: EventHandler<AppConfig>,
    on_close: EventHandler<()>,
) -> Element {
    let mut config_state = use_signal(|| config.clone());
    
    rsx! {
        div { class: "vm-create-overlay",
            div { class: "vm-create-dialog",
                div { class: "vm-create-header",
                    h2 { "Settings" }
                    button {
                        class: "close-button",
                        onclick: move |_| on_close(()),
                        "×"
                    }
                }
                
                div { class: "vm-create-content",
                    div { class: "form-group",
                        label { "VM Directories" }
                        div { class: "directory-list",
                            for (i, dir) in config_state.read().vm_directories.iter().enumerate() {
                                div { class: "directory-item",
                                    span { "{dir.display()}" }
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| {
                                            let mut new_config = (*config_state.read()).clone();
                                            new_config.vm_directories.remove(i);
                                            config_state.set(new_config);
                                        },
                                        "✕ Remove"
                                    }
                                }
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                // In a real app, this would open a file dialog
                                let mut new_config = (*config_state.read()).clone();
                                new_config.vm_directories.push(std::path::PathBuf::from("/home/user/VMs"));
                                config_state.set(new_config);
                            },
                            "📁 Add Directory"
                        }
                    }
                    
                    div { class: "form-group",
                        label { "Tool Management" }
                        div { class: "checkbox-wrapper",
                            input {
                                r#type: "checkbox",
                                id: "auto-download",
                                checked: config_state.read().auto_download_tools,
                                onchange: move |e| {
                                    let mut new_config = (*config_state.read()).clone();
                                    new_config.auto_download_tools = e.value().parse().unwrap_or(false);
                                    config_state.set(new_config);
                                }
                            }
                            label { r#for: "auto-download", "Auto-download quickemu tools if not found" }
                        }
                    }
                    
                    div { class: "form-group",
                        label { r#for: "theme", "Theme" }
                        select {
                            id: "theme",
                            value: "{config_state.read().theme:?}",
                            onchange: move |e| {
                                let mut new_config = (*config_state.read()).clone();
                                new_config.theme = match e.value().as_str() {
                                    "Light" => crate::models::Theme::Light,
                                    "Dark" => crate::models::Theme::Dark,
                                    _ => crate::models::Theme::System,
                                };
                                config_state.set(new_config);
                            },
                            option { value: "System", "🖥 System" }
                            option { value: "Light", "☀ Light" }
                            option { value: "Dark", "🌙 Dark" }
                        }
                    }
                }
                
                div { class: "vm-create-actions",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close(()),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            on_save((*config_state.read()).clone());
                        },
                        "💾 Save Settings"
                    }
                }
            }
        }
    }
}