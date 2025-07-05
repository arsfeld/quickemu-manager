//! WebAssembly bindings for the SPICE client
//!
//! This module provides the JavaScript API for the SPICE client when compiled to WebAssembly.

use crate::{SpiceClientShared, SpiceError};
use std::sync::Arc;
use tokio::sync::Mutex;
use wasm_bindgen::prelude::*;
use web_sys::{console, HtmlCanvasElement};

/// SPICE client for WebAssembly
///
/// This is the main entry point for using the SPICE client in a web browser.
#[wasm_bindgen]
pub struct SpiceClient {
    inner: Arc<Mutex<Option<SpiceClientShared>>>,
    websocket_url: String,
    canvas: Option<HtmlCanvasElement>,
    password: Option<String>,
}

#[wasm_bindgen]
impl SpiceClient {
    /// Create a new SPICE client instance
    ///
    /// # Arguments
    /// * `websocket_url` - The WebSocket URL to connect to (e.g., "ws://localhost:5959")
    /// * `canvas` - The HTML canvas element to render the display
    #[wasm_bindgen(constructor)]
    pub fn new(websocket_url: String, canvas: HtmlCanvasElement) -> Self {
        // Set up console error panic hook for better error messages
        console_error_panic_hook::set_once();

        console::log_1(&format!("Creating new SPICE client for {websocket_url}").into());

        Self {
            inner: Arc::new(Mutex::new(None)),
            websocket_url,
            canvas: Some(canvas),
            password: None,
        }
    }

    /// Create a new SPICE client with password authentication
    ///
    /// # Arguments
    /// * `websocket_url` - The WebSocket URL to connect to
    /// * `canvas` - The HTML canvas element to render the display
    /// * `password` - The SPICE password for authentication
    #[wasm_bindgen(js_name = "newWithPassword")]
    pub fn new_with_password(
        websocket_url: String,
        canvas: HtmlCanvasElement,
        password: String,
    ) -> Self {
        console_error_panic_hook::set_once();

        console::log_1(
            &format!(
                "Creating new SPICE client for {} with password",
                websocket_url
            )
            .into(),
        );

        Self {
            inner: Arc::new(Mutex::new(None)),
            websocket_url,
            canvas: Some(canvas),
            password: Some(password),
        }
    }

    /// Connect to the SPICE server
    #[wasm_bindgen]
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        console::log_1(&format!("Connecting to SPICE server at {}...", self.websocket_url).into());

        // Check if already connected
        if self.inner.lock().await.is_some() {
            console::log_1(&"Already connected".into());
            return Ok(());
        }

        let mut client = SpiceClientShared::new_websocket(self.websocket_url.clone());

        // Set password if provided
        if let Some(ref password) = self.password {
            client.set_password(password.clone()).await;
        }

        match client.connect().await {
            Ok(()) => {
                console::log_1(&"Connected successfully".into());

                // Start the event loop
                match client.start_event_loop().await {
                    Ok(()) => {
                        console::log_1(&"Event loop started".into());
                        *self.inner.lock().await = Some(client);
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to start event loop: {e}");
                        console::error_1(&error_msg.clone().into());
                        Err(JsValue::from_str(&error_msg))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Connection failed: {e}");
                console::error_1(&error_msg.clone().into());
                Err(JsValue::from_str(&error_msg))
            }
        }
    }

    /// Disconnect from the SPICE server
    #[wasm_bindgen]
    pub async fn disconnect(&mut self) {
        console::log_1(&"Disconnecting from SPICE server...".into());

        if let Some(client) = self.inner.lock().await.take() {
            client.disconnect().await;
            console::log_1(&"Disconnected".into());
        }
    }

    /// Get any error state from the client
    #[wasm_bindgen]
    pub async fn get_error(&self) -> Result<Option<String>, JsValue> {
        if let Some(client) = self.inner.lock().await.as_ref() {
            Ok(client.get_error_state().await)
        } else {
            Ok(None)
        }
    }

    /// Send a key event to the server
    ///
    /// This method spawns the actual work to avoid blocking and prevent recursive use errors
    #[wasm_bindgen]
    pub fn send_key(&self, key_code: u32, is_pressed: bool) -> Result<(), JsValue> {
        let inner = self.inner.clone();

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(client) = inner.lock().await.as_ref() {
                // Check for error state first
                if let Some(error) = client.get_error_state().await {
                    console::error_1(
                        &format!("Cannot send key event - client in error state: {error}").into(),
                    );
                    return;
                }

                // TODO: Implement key event sending through the input channel
                console::log_2(
                    &format!("Key event: code={key_code}, pressed={is_pressed}").into(),
                    &JsValue::NULL,
                );
            }
        });

        Ok(())
    }

    /// Send a mouse move event to the server
    ///
    /// This method spawns the actual work to avoid blocking and prevent recursive use errors
    #[wasm_bindgen]
    pub fn send_mouse_move(&self, x: i32, y: i32) -> Result<(), JsValue> {
        let inner = self.inner.clone();

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(client) = inner.lock().await.as_ref() {
                // Check for error state first
                if let Some(_error) = client.get_error_state().await {
                    // Silently ignore mouse moves in error state to avoid log spam
                    return;
                }

                // TODO: Implement mouse move event sending through the input channel
                console::log_2(&format!("Mouse move: x={x}, y={y}").into(), &JsValue::NULL);
            }
        });

        Ok(())
    }

    /// Send a mouse button event to the server
    ///
    /// This method spawns the actual work to avoid blocking and prevent recursive use errors
    #[wasm_bindgen]
    pub fn send_mouse_button(&self, button: u8, is_pressed: bool) -> Result<(), JsValue> {
        let inner = self.inner.clone();

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(client) = inner.lock().await.as_ref() {
                // Check for error state first
                if let Some(error) = client.get_error_state().await {
                    console::error_1(
                        &format!(
                            "Cannot send mouse button event - client in error state: {}",
                            error
                        )
                        .into(),
                    );
                    return;
                }

                // TODO: Implement mouse button event sending through the input channel
                console::log_2(
                    &format!("Mouse button: button={button}, pressed={is_pressed}").into(),
                    &JsValue::NULL,
                );
            }
        });

        Ok(())
    }
}

/// Initialize the WASM module
///
/// This should be called once when the module is loaded.
#[wasm_bindgen(start)]
pub fn main() {
    // Set up better panic messages in the browser console
    console_error_panic_hook::set_once();

    // Initialize logging
    tracing_wasm::set_as_global_default();

    console::log_1(&"SPICE client WASM module initialized".into());
}
