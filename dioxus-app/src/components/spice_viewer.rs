use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use {
    crate::spice_client::{SpiceClient, SpiceMessage, SpiceEvent},
    web_sys::{HtmlCanvasElement, CanvasRenderingContext2d},
    tokio::sync::mpsc,
    wasm_bindgen::{JsValue, JsCast},
};

#[derive(Props, Clone, PartialEq)]
pub struct SpiceViewerProps {
    pub host: String,
    pub port: u16,
    pub auto_connect: bool,
    pub auth_token: Option<String>,
}

#[component]
pub fn SpiceViewer(props: SpiceViewerProps) -> Element {
    let mut connection_state = use_signal(|| SpiceConnectionState::Disconnected);
    let mut error_message = use_signal(|| None::<String>);
    let canvas_id = use_memo(move || format!("spice-canvas-{}", props.port));
    
    #[cfg(target_arch = "wasm32")]
    let mut spice_client_handle = use_signal(|| None::<SpiceClientHandle>);
    
    let host = props.host.clone();
    let port = props.port;
    let ws_url = use_memo(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let protocol = web_sys::window()
                .and_then(|w| w.location().protocol().ok())
                .unwrap_or_else(|| "http:".to_string());
            
            let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
            format!("{}://{}:{}/", ws_protocol, host, port)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            format!("ws://{}:{}/", host, port)
        }
    });

    let auth_token = props.auth_token.clone();
    let auth_token_for_connect = auth_token.clone();
    let _auth_token_for_retry = auth_token.clone();
    let auto_connect = props.auto_connect;
    
    // Initialize SPICE connection when component mounts
    use_effect(move || {
        if auto_connect {
            let ws_url = ws_url();
            let canvas_id = canvas_id();
            let auth_token = auth_token.clone();
            
            spawn(async move {
                connection_state.set(SpiceConnectionState::Connecting);
                
                #[cfg(target_arch = "wasm32")]
                {
                    match connect_spice(&ws_url, &canvas_id, auth_token.clone(), connection_state, error_message).await {
                        Ok(handle) => {
                            spice_client_handle.set(Some(handle));
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to connect: {}", e)));
                            connection_state.set(SpiceConnectionState::Failed);
                        }
                    }
                }
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = (ws_url, canvas_id, auth_token); // Suppress unused warnings
                    error_message.set(Some("SPICE viewer not available in desktop mode".to_string()));
                    connection_state.set(SpiceConnectionState::Failed);
                }
            });
        }
    });
    
    // Cleanup on unmount
    use_drop(move || {
        #[cfg(target_arch = "wasm32")]
        if let Some(handle) = spice_client_handle.take() {
            let _ = handle.message_tx.send(SpiceMessage::Disconnect);
        }
    });

    rsx! {
        div {
            class: "spice-viewer-container h-full w-full bg-black relative",
            
            // SPICE Canvas container
            canvas {
                id: "{canvas_id()}",
                class: "h-full w-full",
                style: if !matches!(connection_state(), SpiceConnectionState::Connected) { "display: none;" } else { "" },
                width: "1024",
                height: "768",
            }
            
            // Show overlay when not connected
            if !matches!(connection_state(), SpiceConnectionState::Connected) {
                div {
                    class: "absolute inset-0 flex items-center justify-center text-white",
                    match connection_state() {
                        SpiceConnectionState::Disconnected => rsx! {
                            div {
                                class: "text-center",
                                p { class: "text-lg mb-4", "SPICE Disconnected" }
                                if !props.auto_connect {
                                    button {
                                        class: "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600",
                                        onclick: move |_| {
                                            let ws_url = ws_url();
                                            let canvas_id = canvas_id();
                                            let auth_token = auth_token_for_connect.clone();
                                            connection_state.set(SpiceConnectionState::Connecting);
                                            
                                            spawn(async move {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    match connect_spice(&ws_url, &canvas_id, auth_token, connection_state, error_message).await {
                                                        Ok(handle) => {
                                                            spice_client_handle.set(Some(handle));
                                                        }
                                                        Err(e) => {
                                                            error_message.set(Some(format!("Failed to connect: {}", e)));
                                                            connection_state.set(SpiceConnectionState::Failed);
                                                        }
                                                    }
                                                }
                                            });
                                        },
                                        "Connect"
                                    }
                                }
                            }
                        },
                        SpiceConnectionState::Connecting => rsx! {
                            div {
                                class: "text-center",
                                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-white mb-4" }
                                p { "Connecting to SPICE server..." }
                                p { class: "text-sm text-gray-400 mt-2", "{ws_url()}" }
                            }
                        },
                        SpiceConnectionState::Failed => rsx! {
                            div {
                                class: "bg-yellow-900 bg-opacity-90 text-yellow-100 p-6 rounded-lg max-w-lg",
                                h3 { class: "text-lg font-semibold mb-2", "SPICE Protocol Detected" }
                                if let Some(error) = error_message() {
                                    p { class: "text-sm mb-4", "{error}" }
                                }
                                
                                div { class: "mt-4 space-y-2 text-sm",
                                    p { class: "font-semibold", "To use the web console:" }
                                    ol { class: "list-decimal list-inside space-y-1 ml-2",
                                        li { "Stop the VM" }
                                        li { "Edit the VM configuration" }
                                        li { "Change display protocol from SPICE to VNC" }
                                        li { "Start the VM again" }
                                    }
                                    
                                    p { class: "mt-3", 
                                        span { class: "font-semibold", "Alternative: " }
                                        "Use a native SPICE client like "
                                        code { class: "bg-yellow-800 px-1 rounded", "spicy" }
                                        " or "
                                        code { class: "bg-yellow-800 px-1 rounded", "remote-viewer" }
                                    }
                                    
                                    p { class: "mt-3 text-xs italic",
                                        "Note: Web-based SPICE support is experimental and may not work with all VMs."
                                    }
                                }
                            }
                        },
                        _ => rsx! {}
                    }
                }
            }
            
            // Connection status indicator
            div {
                class: "absolute top-4 right-4 flex items-center space-x-2 bg-black bg-opacity-50 px-3 py-1 rounded",
                div {
                    class: match connection_state() {
                        SpiceConnectionState::Connected => "w-2 h-2 bg-green-500 rounded-full",
                        SpiceConnectionState::Connecting => "w-2 h-2 bg-yellow-500 rounded-full animate-pulse",
                        SpiceConnectionState::Failed => "w-2 h-2 bg-red-500 rounded-full",
                        SpiceConnectionState::Disconnected => "w-2 h-2 bg-gray-500 rounded-full",
                    }
                }
                span {
                    class: "text-xs text-white",
                    match connection_state() {
                        SpiceConnectionState::Connected => "Connected",
                        SpiceConnectionState::Connecting => "Connecting",
                        SpiceConnectionState::Failed => "Failed",
                        SpiceConnectionState::Disconnected => "Disconnected",
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SpiceConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[cfg(target_arch = "wasm32")]
struct SpiceClientHandle {
    message_tx: mpsc::UnboundedSender<SpiceMessage>,
}

#[cfg(target_arch = "wasm32")]
async fn connect_spice(
    ws_url: &str, 
    canvas_id: &str, 
    auth_token: Option<String>,
    mut connection_state: Signal<SpiceConnectionState>,
    mut error_message: Signal<Option<String>>,
) -> Result<SpiceClientHandle, String> {
    log::info!("SpiceViewer: Starting SPICE connection to {}", ws_url);
    log::debug!("SpiceViewer: Canvas ID: {}, Auth token present: {}", canvas_id, auth_token.is_some());
    
    // Create SPICE client
    let (client, mut event_rx, message_tx) = SpiceClient::new();
    
    // Clone for the event handler
    let canvas_id_clone = canvas_id.to_string();
    let message_tx_clone = message_tx.clone();
    
    // Start client in background
    wasm_bindgen_futures::spawn_local(async move {
        client.run().await;
    });
    
    // Handle events from SPICE client
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                SpiceEvent::Connected => {
                    log::info!("SpiceViewer: Connected to SPICE server");
                    connection_state.set(SpiceConnectionState::Connected);
                }
                SpiceEvent::Disconnected => {
                    log::info!("SpiceViewer: Disconnected from SPICE server");
                    connection_state.set(SpiceConnectionState::Disconnected);
                }
                SpiceEvent::Error(e) => {
                    log::error!("SpiceViewer: Error: {}", e);
                    error_message.set(Some(e));
                    connection_state.set(SpiceConnectionState::Failed);
                }
                SpiceEvent::DisplayUpdate { surface_id, x, y, width, height, data } => {
                    log::debug!("SpiceViewer: Display update for surface {}: {}x{} at ({}, {})", 
                              surface_id, width, height, x, y);
                    // Update canvas with display data
                    if let Err(e) = update_canvas(&canvas_id_clone, x, y, width, height, &data) {
                        log::error!("SpiceViewer: Failed to update canvas: {:?}", e);
                    }
                }
                SpiceEvent::Data(_) => {
                    // Raw data events are handled internally
                }
            }
        }
    });
    
    // Send connect message
    message_tx.send(SpiceMessage::Connect(ws_url.to_string(), auth_token))
        .map_err(|_| "Failed to send connect message".to_string())?;
    
    Ok(SpiceClientHandle { message_tx })
}

#[cfg(target_arch = "wasm32")]
fn update_canvas(canvas_id: &str, x: u32, y: u32, width: u32, height: u32, data: &[u8]) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let canvas = document.get_element_by_id(canvas_id)
        .ok_or("Canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;
    
    let ctx = canvas.get_context("2d")?
        .ok_or("Failed to get 2d context")?
        .dyn_into::<CanvasRenderingContext2d>()?;
    
    // Create ImageData from the raw pixel data
    // Assuming RGBA format for now
    let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(data),
        width,
        height,
    )?;
    
    // Draw the image data to the canvas
    ctx.put_image_data(&image_data, x as f64, y as f64)?;
    
    Ok(())
}