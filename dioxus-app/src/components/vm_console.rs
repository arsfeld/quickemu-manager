use dioxus::prelude::*;

use crate::models::{VM, ConsoleInfo, ConsoleProtocol};
use crate::server_functions::{start_vm_console, stop_vm_console, get_console_status};
use crate::components::vnc_viewer::VncViewer;
use crate::components::spice_viewer::SpiceViewer;

/// Console Component - Supports both VNC and SPICE protocols
#[component]
pub fn VmConsole(vm: VM, on_close: EventHandler<()>, inline_mode: Option<bool>) -> Element {
    let mut console_info = use_signal(|| None::<ConsoleInfo>);
    let mut connection_status = use_signal(|| "disconnected".to_string());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_connecting = use_signal(|| false);
    let inline = inline_mode.unwrap_or(false);
    
    let vm_id = vm.id.clone();
    let vm_name = vm.name.clone();

    // Extract WebSocket connection details from console info
    let (ws_host, ws_port) = if let Some(info) = console_info() {
        let ws_url = &info.websocket_url;
        // Parse WebSocket URL (e.g., "ws://raider.local:6090" or "raider.local:6090")
        if ws_url.contains("://") {
            let without_protocol = ws_url.split("://").nth(1).unwrap_or("localhost:8080");
            let parts: Vec<&str> = without_protocol.split(':').collect();
            let host = parts.get(0).unwrap_or(&"localhost").to_string();
            let port = parts.get(1)
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080);
            (host, port)
        } else {
            let parts: Vec<&str> = ws_url.split(':').collect();
            let host = parts.get(0).unwrap_or(&"localhost").to_string();
            let port = parts.get(1)
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080);
            (host, port)
        }
    } else {
        ("localhost".to_string(), 8080u16)
    };

    // Start console session when component mounts
    use_effect(move || {
        let vm_id_clone = vm_id.clone();
        spawn(async move {
            is_connecting.set(true);
            error_message.set(None);
            
            #[cfg(target_arch = "wasm32")]
            log::info!("VncConsole: Starting console session for VM '{}'", vm_id_clone);
            
            // Get current hostname from browser
            let hostname = {
                #[cfg(target_arch = "wasm32")]
                {
                    let host = web_sys::window()
                        .and_then(|w| w.location().hostname().ok());
                    log::info!("VncConsole: Browser hostname: {:?}", host);
                    host
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    None
                }
            };
            
            #[cfg(target_arch = "wasm32")]
            log::info!("VncConsole: Calling start_vm_console for VM '{}' with hostname: {:?}", 
                     vm_id_clone, hostname);
            
            match start_vm_console(vm_id_clone.clone(), hostname).await {
                Ok(info) => {
                    #[cfg(target_arch = "wasm32")]
                    log::info!("VncConsole: Console started successfully for VM '{}': WebSocket: {}, Connection ID: {}", 
                             vm_id_clone, info.websocket_url, info.connection_id);
                    console_info.set(Some(info));
                },
                Err(e) => {
                    #[cfg(target_arch = "wasm32")]
                    log::error!("VncConsole: Failed to start console for VM '{}': {}", vm_id_clone, e);
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
            
            #[cfg(target_arch = "wasm32")]
            log::info!("VncConsole: Starting status monitoring for connection '{}'", connection_id);
            
            spawn(async move {
                let mut check_count = 0;
                loop {
                    check_count += 1;
                    
                    #[cfg(target_arch = "wasm32")]
                    log::debug!("VncConsole: Checking connection status (check #{})", check_count);
                    
                    match get_console_status(connection_id.clone()).await {
                        Ok(Some(status)) => {
                            #[cfg(target_arch = "wasm32")]
                            log::debug!("VncConsole: Connection status: {}", status);
                            connection_status.set(status);
                        },
                        Ok(None) => {
                            #[cfg(target_arch = "wasm32")]
                            log::warn!("VncConsole: Connection '{}' no longer exists", connection_id);
                            connection_status.set("disconnected".to_string());
                            break;
                        },
                        Err(e) => {
                            #[cfg(target_arch = "wasm32")]
                            log::error!("VncConsole: Error checking connection status: {:?}", e);
                            break;
                        }
                    }
                    
                    // Check status every 2 seconds
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                }
                
                #[cfg(target_arch = "wasm32")]
                log::info!("VncConsole: Status monitoring ended for connection '{}'", connection_id);
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
        if inline {
            // Inline mode for desktop layout
            div { class: "h-full flex flex-col bg-black",
                if is_connecting() {
                    div {
                        class: "flex-1 flex flex-col items-center justify-center",
                        div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-white" }
                        p { class: "mt-4 text-white/70 text-sm", "Connecting to console..." }
                    }
                } else if let Some(error) = error_message() {
                    div {
                        class: "flex-1 flex flex-col items-center justify-center p-4",
                        p { class: "text-red-400 text-sm", "{error}" }
                    }
                } else if let Some(info) = console_info() {
                    match info.protocol {
                        ConsoleProtocol::Vnc => rsx! {
                            crate::components::vnc_viewer::VncViewer {
                                host: ws_host.clone(),
                                port: ws_port,
                                auto_connect: true,
                                auth_token: None,
                            }
                        },
                        ConsoleProtocol::Spice => rsx! {
                            crate::components::spice_viewer::SpiceViewer {
                                host: ws_host.clone(),
                                port: ws_port,
                                password: None,
                                on_status_change: move |status| {
                                    connection_status.set(status);
                                }
                            }
                        }
                    }
                } else {
                    div {
                        class: "flex-1 flex items-center justify-center",
                        p { class: "text-white/50 text-sm", "Console not available" }
                    }
                }
            }
        } else {
            // Modal mode
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
                            h2 { class: "text-lg font-medium", "Console: {vm_name}" }
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
                            h3 { 
                                class: "text-lg font-semibold text-gray-800 mb-4", 
                                match info.protocol {
                                    ConsoleProtocol::Vnc => "VNC Console Connection",
                                    ConsoleProtocol::Spice => "SPICE Console Connection",
                                }
                            }
                            
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
                                div {
                                    class: "flex justify-between text-sm",
                                    span { class: "font-medium text-gray-600", "Protocol:" }
                                    span { class: "text-gray-800 font-medium", 
                                        match info.protocol {
                                            ConsoleProtocol::Vnc => "VNC",
                                            ConsoleProtocol::Spice => "SPICE",
                                        }
                                    }
                                }
                                div {
                                    class: "flex justify-between text-sm",
                                    span { class: "font-medium text-gray-600", "Proxy:" }
                                    span { class: "text-gray-800 font-mono text-xs", "{ws_host}:{ws_port}" }
                                }
                            }

                            // Console Display
                            div {
                                class: "bg-black rounded-lg relative overflow-hidden",
                                style: "min-height: 500px;",
                                
                                if let Some(info) = console_info() {
                                    match info.protocol {
                                        ConsoleProtocol::Vnc => rsx! {
                                            crate::components::vnc_viewer::VncViewer {
                                                host: ws_host,
                                                port: ws_port,
                                                auto_connect: true,
                                                auth_token: Some(info.auth_token.clone())
                                            }
                                        },
                                        ConsoleProtocol::Spice => rsx! {
                                            crate::components::spice_viewer::SpiceViewer {
                                                host: ws_host,
                                                port: ws_port,
                                                password: None,
                                                on_status_change: move |status| {
                                                    connection_status.set(status);
                                                }
                                            }
                                        }
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
                        } else if let Some(info) = console_info() {
                            div { class: "w-2 h-2 bg-green-500 rounded-full" }
                            span { 
                                match info.protocol {
                                    ConsoleProtocol::Vnc => "VNC proxy active",
                                    ConsoleProtocol::Spice => "SPICE proxy active",
                                }
                            }
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
}