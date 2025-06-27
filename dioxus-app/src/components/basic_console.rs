use dioxus::prelude::*;

use crate::models::{VM, ConsoleInfo};
use crate::server_functions::{start_vm_console, stop_vm_console, get_console_status};

/// Basic Console Component - Tests SPICE proxy connection without complex HTML5 client
#[component]
pub fn BasicConsole(vm: VM, on_close: EventHandler<()>) -> Element {
    let mut console_info = use_signal(|| None::<ConsoleInfo>);
    let mut connection_status = use_signal(|| "disconnected".to_string());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_connecting = use_signal(|| false);
    
    let vm_id = vm.id.clone();
    let vm_name = vm.name.clone();

    // Generate iframe URL outside of rsx! macro
    let iframe_url = if let Some(info) = console_info() {
        // Extract host and port from WebSocket URL
        let ws_url = &info.websocket_url;
        let host = if ws_url.contains("://") {
            ws_url.split("://").nth(1)
                .and_then(|s| s.split(":").next())
                .unwrap_or("localhost")
        } else {
            "localhost"
        };
        
        let port = if ws_url.contains(":") {
            ws_url.split(":").last()
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(6080)
        } else {
            6080
        };

        format!(
            "/assets/spice-client/spice-console.html?host={}&port={}&token={}",
            host, port, info.auth_token
        )
    } else {
        String::new()
    };

    // Start console session when component mounts
    use_effect(move || {
        let vm_id_clone = vm_id.clone();
        spawn(async move {
            is_connecting.set(true);
            error_message.set(None);
            
            match start_vm_console(vm_id_clone.clone()).await {
                Ok(info) => {
                    console_info.set(Some(info));
                },
                Err(e) => {
                    error_message.set(Some(format!("Failed to start console: {}", e)));
                }
            }
            is_connecting.set(false);
        });
    });

    // Monitor connection status
    use_effect(move || {
        if let Some(info) = console_info() {
            let connection_id = info.connection_id.clone();
            spawn(async move {
                loop {
                    match get_console_status(connection_id.clone()).await {
                        Ok(Some(status)) => {
                            connection_status.set(status);
                        },
                        Ok(None) => {
                            connection_status.set("disconnected".to_string());
                            break;
                        },
                        Err(_) => {
                            break;
                        }
                    }
                    
                    // Check status every 2 seconds
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                }
            });
        }
    });

    // Cleanup on component unmount
    use_drop(move || {
        if let Some(info) = console_info() {
            spawn(async move {
                let _ = stop_vm_console(info.connection_id).await;
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
                        class: "flex items-center space-x-3",
                        h2 { class: "text-xl font-semibold", "Console: {vm_name}" }
                        div {
                            class: match connection_status().as_str() {
                                "connected" => "w-3 h-3 bg-green-500 rounded-full",
                                "connecting" | "authenticating" => "w-3 h-3 bg-yellow-500 rounded-full animate-pulse",
                                _ => "w-3 h-3 bg-red-500 rounded-full",
                            }
                        }
                        span { 
                            class: "text-sm text-gray-600 capitalize",
                            "{connection_status()}"
                        }
                    }
                    button {
                        class: "text-gray-500 hover:text-gray-800 transition-colors text-2xl",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                // Console content
                div {
                    class: "p-6 bg-gray-50 min-h-96",

                    if is_connecting() {
                        div {
                            class: "flex flex-col items-center justify-center h-64",
                            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                            p { class: "mt-4 text-gray-600", "Connecting to console..." }
                        }
                    } else if let Some(error) = error_message() {
                        div {
                            class: "flex flex-col items-center justify-center h-64 bg-red-100 text-red-800 p-4 rounded-lg",
                            h3 { class: "text-lg font-semibold mb-2", "Console Error" }
                            p { class: "text-sm mb-4", "{error}" }
                            button {
                                class: "btn-macos",
                                onclick: move |_| on_close.call(()),
                                "Close"
                            }
                        }
                    } else if let Some(info) = console_info() {
                        div {
                            class: "space-y-4",
                            h3 { class: "text-lg font-semibold text-gray-800 mb-4", "Console Connection Established" }
                            
                            div {
                                class: "bg-white p-4 rounded-lg border space-y-3",
                                div {
                                    class: "flex justify-between text-sm",
                                    span { class: "font-medium text-gray-600", "Connection ID:" }
                                    span { class: "text-gray-800 font-mono text-xs", "{info.connection_id}" }
                                }
                                div {
                                    class: "flex justify-between text-sm",
                                    span { class: "font-medium text-gray-600", "WebSocket URL:" }
                                    span { class: "text-gray-800 font-mono text-xs", "{info.websocket_url}" }
                                }
                                div {
                                    class: "flex justify-between text-sm",
                                    span { class: "font-medium text-gray-600", "Status:" }
                                    span { 
                                        class: match connection_status().as_str() {
                                            "connected" => "text-green-600 font-medium",
                                            "connecting" | "authenticating" => "text-yellow-600 font-medium",
                                            _ => "text-red-600 font-medium",
                                        },
                                        "{connection_status()}"
                                    }
                                }
                            }

                            // Placeholder for SPICE client integration
                            // Embedded SPICE Console
                            div {
                                class: "bg-black rounded-lg relative overflow-hidden",
                                style: "min-height: 500px;",
                                
                                if !iframe_url.is_empty() {
                                    iframe {
                                        src: "{iframe_url}",
                                        class: "w-full border-0",
                                        style: "height: 500px; background: black;",
                                        title: "SPICE Console"
                                    }
                                } else {
                                    div {
                                        class: "flex flex-col items-center justify-center h-64 text-white",
                                        div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mb-4" }
                                        p { "Initializing console..." }
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
                        if is_connecting() {
                            div { class: "w-2 h-2 bg-blue-500 rounded-full animate-pulse" }
                            span { "Connecting..." }
                        } else if console_info().is_some() {
                            div { class: "w-2 h-2 bg-green-500 rounded-full" }
                            span { "Console proxy active" }
                        } else {
                            div { class: "w-2 h-2 bg-red-500 rounded-full" }
                            span { "Disconnected" }
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