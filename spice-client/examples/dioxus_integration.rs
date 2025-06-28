use dioxus::prelude::*;
use spice_client::{SpiceClient, SpiceError};
use std::sync::Arc;
use tokio::sync::RwLock;
use base64::{engine::general_purpose, Engine as _};

#[derive(Clone)]
struct SpiceState {
    client: Arc<RwLock<Option<SpiceClient>>>,
    surface_data: Arc<RwLock<Option<SurfaceData>>>,
    connected: bool,
    error_message: Option<String>,
}

#[derive(Clone, Debug)]
struct SurfaceData {
    width: u32,
    height: u32,
    data_url: String, // Base64 encoded image data
}

impl Default for SpiceState {
    fn default() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            surface_data: Arc::new(RwLock::new(None)),
            connected: false,
            error_message: None,
        }
    }
}

#[component]
fn SpiceViewer() -> Element {
    let mut spice_state = use_signal(SpiceState::default);
    let mut host = use_signal(|| "localhost".to_string());
    let mut port = use_signal(|| "5900".to_string());

    let connect_to_spice = move |_| {
        let host_val = host.read().clone();
        let port_val: u16 = port.read().parse().unwrap_or(5900);
        let mut state = spice_state.write();
        
        spawn(async move {
            match connect_spice_client(&host_val, port_val).await {
                Ok((client, surface)) => {
                    let mut state = spice_state.write();
                    *state.client.write().await = Some(client);
                    *state.surface_data.write().await = surface;
                    state.connected = true;
                    state.error_message = None;
                }
                Err(e) => {
                    let mut state = spice_state.write();
                    state.connected = false;
                    state.error_message = Some(format!("Connection failed: {}", e));
                }
            }
        });
    };

    let disconnect_spice = move |_| {
        spawn(async move {
            let mut state = spice_state.write();
            if let Some(mut client) = state.client.write().await.take() {
                client.disconnect();
            }
            *state.surface_data.write().await = None;
            state.connected = false;
            state.error_message = None;
        });
    };

    rsx! {
        div { class: "spice-viewer-container p-4",
            div { class: "controls mb-4",
                h2 { class: "text-xl font-bold mb-2", "SPICE Client" }
                
                div { class: "flex gap-2 mb-2",
                    input {
                        class: "border p-2 rounded",
                        placeholder: "Host",
                        value: "{host}",
                        oninput: move |e| host.set(e.value())
                    }
                    input {
                        class: "border p-2 rounded",
                        placeholder: "Port",
                        value: "{port}",
                        oninput: move |e| port.set(e.value())
                    }
                }
                
                div { class: "flex gap-2",
                    if !spice_state.read().connected {
                        button {
                            class: "bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600",
                            onclick: connect_to_spice,
                            "Connect"
                        }
                    } else {
                        button {
                            class: "bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600",
                            onclick: disconnect_spice,
                            "Disconnect"
                        }
                    }
                }
                
                if let Some(error) = &spice_state.read().error_message {
                    div { class: "text-red-500 mt-2", "{error}" }
                }
            }
            
            div { class: "display-area border rounded",
                if spice_state.read().connected {
                    SpiceDisplay { surface_data: spice_state.read().surface_data.clone() }
                } else {
                    div { class: "p-8 text-center text-gray-500",
                        "Not connected to SPICE server"
                    }
                }
            }
        }
    }
}

#[component]
fn SpiceDisplay(surface_data: Arc<RwLock<Option<SurfaceData>>>) -> Element {
    let surface = use_resource(move || {
        let surface_data = surface_data.clone();
        async move {
            surface_data.read().await.clone()
        }
    });

    match &*surface.read_unchecked() {
        Some(Some(data)) => rsx! {
            div { class: "relative",
                img {
                    src: "{data.data_url}",
                    alt: "SPICE Display",
                    class: "max-w-full h-auto border",
                    width: "{data.width}",
                    height: "{data.height}"
                }
                div { class: "absolute bottom-2 right-2 bg-black bg-opacity-50 text-white px-2 py-1 rounded text-sm",
                    "{data.width}x{data.height}"
                }
            }
        },
        _ => rsx! {
            div { class: "p-8 text-center",
                div { class: "animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-2" }
                "Loading display..."
            }
        }
    }
}

async fn connect_spice_client(host: &str, port: u16) -> Result<(SpiceClient, Option<SurfaceData>), SpiceError> {
    let mut client = SpiceClient::new(host.to_string(), port);
    client.connect().await?;
    
    // Start the event loop in the background
    client.start_event_loop().await?;
    
    // Give it a moment to establish display
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Try to get display surface
    let surface_data = if let Some(surface) = client.get_display_surface(0).await {
        // Convert raw pixel data to base64 image
        let data_url = create_image_data_url(surface.width, surface.height, &surface.data, surface.format);
        Some(SurfaceData {
            width: surface.width,
            height: surface.height,
            data_url,
        })
    } else {
        None
    };
    
    Ok((client, surface_data))
}

fn create_image_data_url(width: u32, height: u32, data: &[u8], format: u32) -> String {
    // This is a simplified implementation
    // In a real implementation, you'd need to handle different pixel formats
    // and convert to a web-compatible format like PNG or JPEG
    
    // For now, assume RGBA format and create a simple data URL
    if data.len() >= (width * height * 4) as usize {
        // Create a simple bitmap header for demonstration
        let encoded = general_purpose::STANDARD.encode(data);
        format!("data:image/png;base64,{}", encoded)
    } else {
        // Fallback: create a placeholder image
        "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMzIwIiBoZWlnaHQ9IjI0MCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cmVjdCB3aWR0aD0iMTAwJSIgaGVpZ2h0PSIxMDAlIiBmaWxsPSIjY2NjIi8+PHRleHQgeD0iNTAlIiB5PSI1MCUiIGZvbnQtZmFtaWx5PSJBcmlhbCIgZm9udC1zaXplPSIxNCIgZmlsbD0iIzMzMyIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZHk9Ii4zZW0iPk5vIERpc3BsYXk8L3RleHQ+PC9zdmc+".to_string()
    }
}

fn main() {
    dioxus::launch(SpiceViewer);
}