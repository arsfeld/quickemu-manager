use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use {
    crate::vnc_client::{VncClient, VncMessage, VncEvent},
    crate::vnc_protocol::{VncProtocolHandler, PixelFormat},
    web_sys::HtmlCanvasElement,
    tokio::sync::mpsc,
    wasm_bindgen::JsValue,
};

#[derive(Props, Clone, PartialEq)]
pub struct VncViewerProps {
    pub host: String,
    pub port: u16,
    pub auto_connect: bool,
    pub auth_token: Option<String>,
}

#[component]
pub fn VncViewer(props: VncViewerProps) -> Element {
    let mut connection_state = use_signal(|| VncConnectionState::Disconnected);
    let mut error_message = use_signal(|| None::<String>);
    let canvas_id = use_memo(move || format!("vnc-canvas-{}", props.port));
    
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
    let auth_token_for_retry = auth_token.clone();
    let auto_connect = props.auto_connect;
    
    // Initialize VNC connection when component mounts
    use_effect(move || {
        if auto_connect {
            let ws_url = ws_url();
            let canvas_id = canvas_id();
            let auth_token = auth_token.clone();
            
            spawn(async move {
                connection_state.set(VncConnectionState::Connecting);
                
                #[cfg(target_arch = "wasm32")]
                {
                    match connect_vnc(&ws_url, &canvas_id, auth_token.clone()).await {
                        Ok(_) => {
                            connection_state.set(VncConnectionState::Connected);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to connect: {}", e)));
                            connection_state.set(VncConnectionState::Failed);
                        }
                    }
                }
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = (ws_url, canvas_id, auth_token); // Suppress unused warnings
                    error_message.set(Some("VNC viewer not available in desktop mode".to_string()));
                    connection_state.set(VncConnectionState::Failed);
                }
            });
        }
    });

    rsx! {
        div {
            class: "vnc-viewer-container h-full w-full bg-black relative",
            
            // VNC Canvas container
            div {
                id: "{canvas_id()}",
                class: "h-full w-full",
                style: if !matches!(connection_state(), VncConnectionState::Connected) { "display: none;" } else { "" },
                // Canvas will be created dynamically by the VNC protocol handler
            }
            
            // Show overlay when not connected
            if !matches!(connection_state(), VncConnectionState::Connected) {
                div {
                    class: "absolute inset-0 flex items-center justify-center text-white",
                    match connection_state() {
                        VncConnectionState::Disconnected => rsx! {
                            div {
                                class: "text-center",
                                p { class: "text-lg mb-4", "VNC Disconnected" }
                                if !props.auto_connect {
                                    button {
                                        class: "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600",
                                        onclick: move |_| {
                                            let ws_url = ws_url();
                                            let canvas_id = canvas_id();
                                            let auth_token = auth_token_for_connect.clone();
                                            connection_state.set(VncConnectionState::Connecting);
                                            
                                            spawn(async move {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    match connect_vnc(&ws_url, &canvas_id, auth_token).await {
                                                        Ok(_) => {
                                                            connection_state.set(VncConnectionState::Connected);
                                                        }
                                                        Err(e) => {
                                                            error_message.set(Some(format!("Failed to connect: {}", e)));
                                                            connection_state.set(VncConnectionState::Failed);
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
                        VncConnectionState::Connecting => rsx! {
                            div {
                                class: "text-center",
                                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-white mb-4" }
                                p { "Connecting to VNC server..." }
                                p { class: "text-sm text-gray-400 mt-2", "{ws_url()}" }
                            }
                        },
                        VncConnectionState::Failed => rsx! {
                            div {
                                class: "bg-red-900 bg-opacity-90 text-red-100 p-6 rounded-lg max-w-md",
                                h3 { class: "text-lg font-semibold mb-2", "Connection Failed" }
                                if let Some(error) = error_message() {
                                    p { class: "text-sm mb-4", "{error}" }
                                }
                                button {
                                    class: "px-4 py-2 bg-red-700 text-white rounded hover:bg-red-600",
                                    onclick: move |_| {
                                        let ws_url = ws_url();
                                        let canvas_id = canvas_id();
                                        let auth_token = auth_token_for_retry.clone();
                                        connection_state.set(VncConnectionState::Connecting);
                                        error_message.set(None);
                                        
                                        spawn(async move {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                match connect_vnc(&ws_url, &canvas_id, auth_token).await {
                                                    Ok(_) => {
                                                        connection_state.set(VncConnectionState::Connected);
                                                    }
                                                    Err(e) => {
                                                        error_message.set(Some(format!("Failed to connect: {}", e)));
                                                        connection_state.set(VncConnectionState::Failed);
                                                    }
                                                }
                                            }
                                        });
                                    },
                                    "Retry"
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
                        VncConnectionState::Connected => "w-2 h-2 bg-green-500 rounded-full",
                        VncConnectionState::Connecting => "w-2 h-2 bg-yellow-500 rounded-full animate-pulse",
                        VncConnectionState::Failed => "w-2 h-2 bg-red-500 rounded-full",
                        VncConnectionState::Disconnected => "w-2 h-2 bg-gray-500 rounded-full",
                    }
                }
                span {
                    class: "text-xs text-white",
                    match connection_state() {
                        VncConnectionState::Connected => "Connected",
                        VncConnectionState::Connecting => "Connecting",
                        VncConnectionState::Failed => "Failed",
                        VncConnectionState::Disconnected => "Disconnected",
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum VncConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[cfg(target_arch = "wasm32")]
async fn connect_vnc(ws_url: &str, canvas_id: &str, auth_token: Option<String>) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    
    log::info!("VncViewer: Starting VNC connection to {}", ws_url);
    log::debug!("VncViewer: Canvas ID: {}, Auth token present: {}", canvas_id, auth_token.is_some());
    
    // Create protocol handler and set up canvas
    let mut protocol_handler = VncProtocolHandler::new();
    protocol_handler.set_canvas(canvas_id)
        .map_err(|e| {
            log::error!("VncViewer: Failed to set canvas: {:?}", e);
            format!("Failed to set canvas: {:?}", e)
        })?;
    
    log::debug!("VncViewer: Canvas setup completed");
    
    // Create VNC client
    let (client, mut event_rx, message_tx) = VncClient::new();
    
    log::debug!("VncViewer: VNC client created");
    
    // Run client in background
    wasm_bindgen_futures::spawn_local(async move {
        log::debug!("VncViewer: Starting VNC client event loop");
        client.run().await;
        log::debug!("VncViewer: VNC client event loop ended");
    });
    
    // Connect to WebSocket
    log::info!("VncViewer: Sending connect message to WebSocket");
    message_tx.send(VncMessage::Connect(ws_url.to_string(), auth_token))
        .map_err(|e| {
            log::error!("VncViewer: Failed to send connect message: {:?}", e);
            "Failed to send connect message"
        })?;
    
    // Handle VNC protocol in background
    wasm_bindgen_futures::spawn_local({
        let message_tx = message_tx.clone();
        async move {
            let mut connected = false;
            let mut protocol_handler = protocol_handler;
            let mut event_count = 0;
            
            log::debug!("VncViewer: Starting VNC protocol handler");
            
            while let Some(event) = event_rx.recv().await {
                event_count += 1;
                log::debug!("VncViewer: Received event #{}: {:?}", event_count, 
                          match &event {
                              VncEvent::Connected => "Connected",
                              VncEvent::Disconnected => "Disconnected",
                              VncEvent::Error(_) => "Error",
                              VncEvent::Data(_) => "Data",
                          });
                
                match event {
                    VncEvent::Connected => {
                        log::info!("VncViewer: VNC WebSocket connected successfully");
                        connected = true;
                        
                        // Send initial VNC handshake
                        // For now, we'll just request a framebuffer update to get started
                        let pixel_format = PixelFormat::default();
                        log::debug!("VncViewer: Sending initial VNC handshake");
                        let _ = message_tx.send(VncMessage::SendData(protocol_handler.set_pixel_format(&pixel_format)));
                        let _ = message_tx.send(VncMessage::SendData(protocol_handler.set_encodings(&[0]))); // Raw encoding
                        let _ = message_tx.send(VncMessage::SendData(protocol_handler.framebuffer_update_request(false, 0, 0, 800, 600)));
                        log::debug!("VncViewer: Initial VNC handshake sent");
                    }
                VncEvent::Disconnected => {
                    log::info!("VncViewer: VNC WebSocket disconnected");
                    connected = false;
                    break;
                }
                VncEvent::Error(e) => {
                    log::error!("VncViewer: VNC error: {}", e);
                    break;
                }
                VncEvent::Data(data) => {
                    if connected {
                        log::debug!("VncViewer: Received {} bytes of VNC data", data.len());
                        match protocol_handler.handle_server_message(&data).await {
                            Ok(response) => {
                                if !response.is_empty() {
                                    log::debug!("VncViewer: Sending {} bytes response", response.len());
                                    let _ = message_tx.send(VncMessage::SendData(response));
                                }
                            }
                            Err(e) => {
                                log::error!("VncViewer: Failed to handle server message: {}", e);
                            }
                        }
                    } else {
                        log::warn!("VncViewer: Received data while not connected, ignoring");
                    }
                }
            }
            }
            
            log::info!("VncViewer: VNC protocol handler ended");
        }
    });
    
    Ok(())
}