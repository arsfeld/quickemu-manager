use crate::models::{AppConfigDto, ThemeDto};
use crate::server_functions::{
    get_app_config, get_vm_directory, set_vm_directory, update_app_config,
};
use dioxus::prelude::*;

/// Settings Modal Component
#[component]
pub fn SettingsModal(show: Signal<bool>) -> Element {
    // Settings state
    let mut vm_directory = use_signal(|| "/home/user/VMs".to_string());
    let mut theme = use_signal(|| "dark".to_string());
    let mut show_advanced = use_signal(|| false);

    // Load saved settings on mount
    use_effect(move || {
        spawn(async move {
            // Load configuration from server
            if let Ok(config) = get_app_config().await {
                vm_directory.set(config.get_primary_vm_directory());
                theme.set(match config.theme {
                    ThemeDto::Light => "light".to_string(),
                    ThemeDto::Dark => "dark".to_string(),
                    ThemeDto::System => "auto".to_string(),
                });
            } else {
                // Fallback to loading VM directory only
                if let Ok(dir) = get_vm_directory().await {
                    vm_directory.set(dir);
                }
            }

            // Load other settings from localStorage (web-specific settings)
            load_settings(vm_directory, theme);
        });
    });

    if !show() {
        return rsx! { div {} };
    }

    rsx! {
        div {
            class: "fixed inset-0 bg-black/30 flex items-center justify-center z-50 p-4",
            onclick: move |_| show.set(false),

            div {
                class: "modal-macos w-full max-w-2xl",
                onclick: move |e| e.stop_propagation(),

                // Header
                div { class: "border-b border-macos-border px-6 py-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-xl font-semibold text-gray-900", "Settings" }
                        button {
                            class: "text-gray-400 hover:text-gray-600 transition-colors",
                            onclick: move |_| show.set(false),
                            svg { class: "w-5 h-5",
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M6 18L18 6M6 6l12 12"
                                }
                            }
                        }
                    }
                }

                // Content
                div { class: "p-6 bg-white space-y-6 max-h-[60vh] overflow-y-auto",

                    // VM Directory Settings
                    div { class: "space-y-3",
                        h3 { class: "text-lg font-medium text-gray-900", "Virtual Machine Storage" }

                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-2", "VM Directory" }
                            div { class: "flex gap-2",
                                input {
                                    class: "input-macos flex-1",
                                    value: vm_directory(),
                                    placeholder: "/home/arosenfeld/VMs",
                                    oninput: move |e| vm_directory.set(e.value()),
                                }
                                button {
                                    class: "btn-macos px-3",
                                    onclick: move |_| {
                                        // TODO: Open directory picker
                                    },
                                    "Browse"
                                }
                            }
                            p { class: "text-xs text-gray-500 mt-1",
                                "Default location where new VMs will be created"
                            }
                        }
                    }

                    // Theme Settings
                    div { class: "space-y-3",
                        h3 { class: "text-lg font-medium text-gray-900", "Appearance" }

                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-2", "Theme" }
                            select {
                                class: "input-macos",
                                value: theme(),
                                onchange: move |e| theme.set(e.value()),
                                option { value: "dark", "Dark" }
                                option { value: "light", "Light" }
                                option { value: "auto", "System" }
                            }
                        }
                    }

                    // Advanced Settings Toggle
                    div { class: "border-t border-gray-200 pt-6",
                        button {
                            class: "flex items-center gap-2 text-sm text-gray-600 hover:text-gray-800 transition-colors",
                            onclick: move |_| show_advanced.set(!show_advanced()),
                            svg { class: format!("w-4 h-4 transition-transform {}", if show_advanced() { "rotate-90" } else { "" }),
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M9 5l7 7-7 7"
                                }
                            }
                            "Advanced Settings"
                        }

                        if show_advanced() {
                            div { class: "mt-4 space-y-4 pl-6 border-l-2 border-gray-100",

                                // SPICE Settings
                                div {
                                    h4 { class: "font-medium text-gray-800 mb-2", "Console Settings" }
                                    div { class: "space-y-2",
                                        div { class: "flex items-center gap-3",
                                            input {
                                                r#type: "checkbox",
                                                class: "accent-blue-500",
                                                checked: true,
                                            }
                                            label { class: "text-sm text-gray-700", "Enable SPICE console" }
                                        }
                                        div { class: "flex items-center gap-3",
                                            input {
                                                r#type: "checkbox",
                                                class: "accent-blue-500",
                                                checked: false,
                                            }
                                            label { class: "text-sm text-gray-700", "Auto-connect to console on VM start" }
                                        }
                                    }
                                }

                                // Debug Settings
                                div {
                                    h4 { class: "font-medium text-gray-800 mb-2", "Debug" }
                                    div { class: "space-y-2",
                                        div { class: "flex items-center gap-3",
                                            input {
                                                r#type: "checkbox",
                                                class: "accent-blue-500",
                                                checked: false,
                                            }
                                            label { class: "text-sm text-gray-700", "Enable verbose logging" }
                                        }
                                        div { class: "flex items-center gap-3",
                                            input {
                                                r#type: "checkbox",
                                                class: "accent-blue-500",
                                                checked: false,
                                            }
                                            label { class: "text-sm text-gray-700", "Show quickemu command output" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer
                div { class: "border-t border-gray-200 bg-gray-50 px-6 py-4 flex justify-between items-center",
                    button {
                        class: "btn-macos",
                        onclick: move |_| {
                            // Reset to defaults
                            spawn(async move {
                                // For all platforms, reset to simple defaults
                                if let Ok(dir) = get_vm_directory().await {
                                    vm_directory.set(dir);
                                }
                                theme.set("dark".to_string());
                            });
                        },
                        "Reset to Defaults"
                    }

                    div { class: "flex space-x-3",
                        button {
                            class: "btn-macos",
                            onclick: move |_| show.set(false),
                            "Cancel"
                        }

                        button {
                            class: "btn-macos-primary",
                            onclick: move |_| {
                                spawn(async move {
                                    // Save VM directory to server config
                                    if let Err(_e) = set_vm_directory(vm_directory()).await {
                                        // Error handled by server
                                    }

                                    // Save other settings (theme) to config
                                    if let Ok(mut config) = get_app_config().await {
                                        config.theme = match theme().as_str() {
                                            "light" => ThemeDto::Light,
                                            "dark" => ThemeDto::Dark,
                                            _ => ThemeDto::System,
                                        };

                                        if let Err(_e) = update_app_config(config).await {
                                            // Error handled by server
                                        }
                                    }

                                    // Also save web-specific settings to localStorage
                                    save_settings(
                                        vm_directory(),
                                        theme()
                                    );
                                });
                                show.set(false);
                            },
                            "Save Settings"
                        }
                    }
                }
            }
        }
    }
}

/// Load settings from localStorage (web)
fn load_settings(mut _vm_directory: Signal<String>, mut _theme: Signal<String>) {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(dir)) = storage.get_item("vm_directory") {
                    _vm_directory.set(dir);
                }
                if let Ok(Some(theme_val)) = storage.get_item("theme") {
                    _theme.set(theme_val);
                }
            }
        }
    }
}

/// Save settings to localStorage (web)
fn save_settings(vm_directory: String, theme: String) {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("vm_directory", &vm_directory);
                let _ = storage.set_item("theme", &theme);
            }
        }
    }
}
