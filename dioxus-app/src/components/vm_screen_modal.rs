use dioxus::prelude::*;

use crate::models::VM;
use crate::server_functions::get_vm_screenshot;
use crate::components::vnc_viewer::VncViewer;

/// VM Screen Modal Component
#[component]
pub fn VMScreenModal(vm: VM, on_close: EventHandler<()>) -> Element {
    let mut screenshot_data = use_signal(|| None::<String>);
    let mut error_message = use_signal(|| None::<String>);
    let mut is_loading = use_signal(|| true);
    let mut display_mode = use_signal(|| DisplayMode::VNC);
    let vm_id = vm.id.clone();

    #[derive(Clone, PartialEq)]
    enum DisplayMode {
        VNC,
        Screenshot,
    }

    // Fetch screenshot when in screenshot mode
    use_effect(move || {
        let vm_id_clone = vm_id.clone();
        let display_mode = display_mode.read().clone();
        
        if display_mode == DisplayMode::Screenshot {
            spawn(async move {
                loop {
                    is_loading.set(true);
                    match get_vm_screenshot(vm_id_clone.clone()).await {
                        Ok(data) => {
                            screenshot_data.set(Some(data));
                            error_message.set(None);
                        },
                        Err(e) => {
                            error_message.set(Some(format!("Failed to get screenshot: {}", e)));
                        }
                    }
                    is_loading.set(false);

                    // Refresh every 2 seconds
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                }
            });
        }
    });

    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50",
            onclick: move |_| on_close.call(()),

            div {
                class: "modal-macos p-0 max-w-4xl w-full",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-macos-border",
                    div {
                        h2 { class: "text-xl font-semibold", "VM Screen: {vm.name}" }
                        // Display mode tabs
                        div { class: "flex mt-2 space-x-2",
                            button {
                                class: if *display_mode.read() == DisplayMode::VNC { 
                                    "px-3 py-1 text-sm bg-blue-500 text-white rounded"
                                } else { 
                                    "px-3 py-1 text-sm bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                                },
                                onclick: move |_| display_mode.set(DisplayMode::VNC),
                                "VNC Display"
                            }
                            button {
                                class: if *display_mode.read() == DisplayMode::Screenshot { 
                                    "px-3 py-1 text-sm bg-blue-500 text-white rounded"
                                } else { 
                                    "px-3 py-1 text-sm bg-gray-200 text-gray-700 rounded hover:bg-gray-300"
                                },
                                onclick: move |_| display_mode.set(DisplayMode::Screenshot),
                                "Screenshot Mode"
                            }
                        }
                    }
                    button {
                        class: "text-gray-500 hover:text-gray-800 transition-colors text-2xl",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                // Screen content
                div {
                    class: "bg-black",
                    style: "height: 600px;",
                    
                    match *display_mode.read() {
                        DisplayMode::VNC => rsx! {
                            VncViewer {
                                host: "localhost".to_string(),
                                port: 5900,
                                auto_connect: true,
                                auth_token: None
                            }
                        },
                        DisplayMode::Screenshot => rsx! {
                            div { class: "p-4 h-full",
                                if is_loading() && screenshot_data().is_none() {
                                    div {
                                        class: "flex flex-col items-center justify-center h-full",
                                        div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                                        p { class: "text-white mt-4", "Loading screen..." }
                                    }
                                } else if let Some(error) = error_message() {
                                    div {
                                        class: "flex flex-col items-center justify-center h-full bg-red-100 text-red-800 p-4 rounded-lg",
                                        p { "{error}" }
                                    }
                                } else if let Some(data) = screenshot_data() {
                                    img {
                                        src: "{data}",
                                        class: "w-full h-auto object-contain rounded-lg"
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Footer
                div {
                    class: "p-4 bg-gray-50 border-t border-macos-border flex items-center justify-between",
                    div {
                        class: "flex items-center space-x-2 text-sm text-gray-600",
                        match *display_mode.read() {
                            DisplayMode::VNC => rsx! {
                                div { class: "w-2 h-2 bg-purple-500 rounded-full" }
                                span { "VNC Protocol - Real-time display" }
                            },
                            DisplayMode::Screenshot => rsx! {
                                if is_loading() {
                                    div { class: "w-2 h-2 bg-blue-500 rounded-full animate-pulse" }
                                    span { "Updating..." }
                                } else {
                                    div { class: "w-2 h-2 bg-green-500 rounded-full" }
                                    span { "Live (refreshes every 2s)" }
                                }
                            }
                        }
                    }
                    button {
                        class: "btn-macos",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                }
            }
        }
    }
}
