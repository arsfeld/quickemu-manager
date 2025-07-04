use dioxus::prelude::*;
use dioxus::events::{KeyboardData, MouseData, WheelData};

#[cfg(target_arch = "wasm32")]
use {
    crate::spice_client_wrapper::{SpiceClientWrapper, SpiceMessage, SpiceEvent},
    web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, KeyboardEvent, MouseEvent, WheelEvent},
    tokio::sync::mpsc,
    wasm_bindgen::{JsValue, JsCast},
    wasm_bindgen_futures::spawn_local,
};

#[derive(Props, Clone, PartialEq)]
pub struct SpiceViewerProps {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub on_status_change: Option<EventHandler<String>>,
}

#[component]
pub fn SpiceViewer(props: SpiceViewerProps) -> Element {
    let mut connection_state = use_signal(|| SpiceConnectionState::Disconnected);
    let mut error_message = use_signal(|| None::<String>);
    let mut display_size = use_signal(|| (800u32, 600u32));
    let canvas_id = use_memo(move || format!("spice-canvas-{}", props.port));
    
    #[cfg(target_arch = "wasm32")]
    let mut spice_handle = use_signal(|| None::<SpiceHandle>);
    
    let host = props.host.clone();
    let port = props.port;
    let password = props.password.clone();
    
    // Build WebSocket URL
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
            format!("{}:{}", host, port)
        }
    });
    
    // Initialize SPICE connection when component mounts
    use_effect(move || {
        let ws_url = ws_url();
        let canvas_id = canvas_id();
        let password = password.clone();
        
        spawn(async move {
            connection_state.set(SpiceConnectionState::Connecting);
            
            #[cfg(target_arch = "wasm32")]
            {
                match connect_spice(&ws_url, &canvas_id, password, connection_state, error_message, display_size).await {
                    Ok(handle) => {
                        spice_handle.set(Some(handle));
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to connect: {}", e)));
                        connection_state.set(SpiceConnectionState::Failed);
                    }
                }
            }
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (ws_url, canvas_id, password); // Suppress unused warnings
                error_message.set(Some("SPICE viewer not available in desktop mode".to_string()));
                connection_state.set(SpiceConnectionState::Failed);
            }
        });
    });
    
    // Report status changes
    use_effect(move || {
        if let Some(handler) = &props.on_status_change {
            let status = match connection_state() {
                SpiceConnectionState::Connected => "connected",
                SpiceConnectionState::Connecting => "connecting",
                SpiceConnectionState::Failed => "failed",
                SpiceConnectionState::Disconnected => "disconnected",
            };
            handler.call(status.to_string());
        }
    });
    
    // Cleanup on unmount
    use_drop(move || {
        #[cfg(target_arch = "wasm32")]
        if let Some(handle) = spice_handle.take() {
            let _ = handle.message_tx.send(SpiceMessage::Disconnect);
        }
    });

    rsx! {
        div {
            class: "spice-viewer-container h-full w-full bg-black relative",
            
            // SPICE Canvas
            canvas {
                id: "{canvas_id()}",
                class: "h-full w-full object-contain",
                style: "image-rendering: pixelated; image-rendering: -moz-crisp-edges;",
                width: "{display_size().0}",
                height: "{display_size().1}",
                tabindex: "0",
                
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
            }
            
            // Connection status overlay
            if !matches!(connection_state(), SpiceConnectionState::Connected) {
                div {
                    class: "absolute inset-0 flex items-center justify-center bg-black bg-opacity-75",
                    match connection_state() {
                        SpiceConnectionState::Connecting => rsx! {
                            div {
                                class: "text-center",
                                div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-white mb-4" }
                                p { class: "text-white text-sm", "Connecting to SPICE server..." }
                            }
                        },
                        SpiceConnectionState::Failed => rsx! {
                            div {
                                class: "text-center",
                                p { class: "text-red-400 text-sm mb-2", "Connection failed" }
                                if let Some(error) = error_message() {
                                    p { class: "text-white text-xs", "{error}" }
                                }
                            }
                        },
                        SpiceConnectionState::Disconnected => rsx! {
                            div {
                                class: "text-center",
                                p { class: "text-white text-sm", "Disconnected" }
                            }
                        },
                        _ => rsx! {}
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
struct SpiceHandle {
    message_tx: mpsc::UnboundedSender<SpiceMessage>,
}

#[cfg(target_arch = "wasm32")]
async fn connect_spice(
    ws_url: &str, 
    canvas_id: &str, 
    password: Option<String>,
    mut connection_state: Signal<SpiceConnectionState>,
    mut error_message: Signal<Option<String>>,
    mut display_size: Signal<(u32, u32)>,
) -> Result<SpiceHandle, String> {
    log::info!("SpiceViewer: Connecting to {}", ws_url);
    
    // Create SPICE client wrapper
    let (mut wrapper, mut event_rx, message_tx) = SpiceClientWrapper::new();
    
    // Set up display update callback
    let canvas_id_clone = canvas_id.to_string();
    wrapper.set_update_callback(move |surface_id, surface| {
        log::debug!("Display callback: surface {} - {}x{}", surface_id, surface.width, surface.height);
        
        // Update canvas with surface data
        if let Err(e) = update_canvas(&canvas_id_clone, surface) {
            log::error!("Failed to update canvas: {:?}", e);
        }
    });
    
    // Start client in background
    spawn_local(async move {
        wrapper.run().await;
    });
    
    // Handle events from SPICE client
    let canvas_id = canvas_id.to_string();
    spawn_local(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                SpiceEvent::Connected => {
                    log::info!("SPICE connected");
                    connection_state.set(SpiceConnectionState::Connected);
                }
                SpiceEvent::Disconnected => {
                    log::info!("SPICE disconnected");
                    connection_state.set(SpiceConnectionState::Disconnected);
                }
                SpiceEvent::Error(e) => {
                    log::error!("SPICE error: {}", e);
                    error_message.set(Some(e));
                    connection_state.set(SpiceConnectionState::Failed);
                }
                SpiceEvent::DisplayUpdate { surface_id, surface } => {
                    log::debug!("Display update: surface {} - {}x{}", surface_id, surface.width, surface.height);
                    
                    // Update display size
                    display_size.set((surface.width, surface.height));
                    
                    // Update canvas
                    if let Err(e) = update_canvas(&canvas_id, &surface) {
                        log::error!("Failed to update canvas: {:?}", e);
                    }
                }
                SpiceEvent::CursorUpdate { x, y, visible } => {
                    log::debug!("Cursor update: ({}, {}) visible={}", x, y, visible);
                    // TODO: Implement cursor rendering
                }
            }
        }
    });
    
    // Send connect message
    message_tx.send(SpiceMessage::Connect(ws_url.to_string(), password))
        .map_err(|_| "Failed to send connect message".to_string())?;
    
    Ok(SpiceHandle { message_tx })
}

#[cfg(target_arch = "wasm32")]
fn update_canvas(canvas_id: &str, surface: &spice_client::channels::DisplaySurface) -> Result<(), JsValue> {
    // SPICE bitmap format constants
    const SPICE_BITMAP_FMT_16BIT: u32 = 6;
    const SPICE_BITMAP_FMT_24BIT: u32 = 7;
    const SPICE_BITMAP_FMT_32BIT: u32 = 8;
    
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let canvas = document.get_element_by_id(canvas_id)
        .ok_or("Canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;
    
    // Update canvas size if needed
    if canvas.width() != surface.width || canvas.height() != surface.height {
        canvas.set_width(surface.width);
        canvas.set_height(surface.height);
    }
    
    let ctx = canvas.get_context("2d")?
        .ok_or("Failed to get 2d context")?
        .dyn_into::<CanvasRenderingContext2d>()?;
    
    // Convert surface data to RGBA format if needed
    let rgba_data = match surface.format {
        SPICE_BITMAP_FMT_32BIT => {
            // Already in RGBA format
            surface.data.clone()
        }
        SPICE_BITMAP_FMT_24BIT => {
            // Convert RGB to RGBA
            let mut rgba = Vec::with_capacity(surface.data.len() * 4 / 3);
            for chunk in surface.data.chunks(3) {
                if chunk.len() == 3 {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(255);      // A
                }
            }
            rgba
        }
        SPICE_BITMAP_FMT_16BIT => {
            // Convert RGB565 to RGBA
            let mut rgba = Vec::with_capacity(surface.data.len() * 2);
            for chunk in surface.data.chunks(2) {
                if chunk.len() == 2 {
                    let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let r = ((pixel >> 11) & 0x1F) << 3;
                    let g = ((pixel >> 5) & 0x3F) << 2;
                    let b = (pixel & 0x1F) << 3;
                    rgba.push(r as u8);
                    rgba.push(g as u8);
                    rgba.push(b as u8);
                    rgba.push(255);
                }
            }
            rgba
        }
        _ => {
            log::warn!("Unsupported surface format: {}", surface.format);
            return Ok(());
        }
    };
    
    // Create ImageData from the RGBA data
    let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&rgba_data),
        surface.width,
        surface.height,
    )?;
    
    // Draw the image data to the canvas
    ctx.put_image_data(&image_data, 0.0, 0.0)?;
    
    Ok(())
}