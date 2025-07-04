use super::{Transport, TransportConfig};
use crate::error::{Result, SpiceError};
use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use tokio::sync::mpsc;
use tracing::{error, info};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

/// WebSocket transport implementation for WASM
pub struct WebSocketTransport {
    ws: WebSocket,
    rx: mpsc::Receiver<Vec<u8>>,
    read_buffer: BytesMut,
    is_connected: Rc<RefCell<bool>>,
}

impl WebSocketTransport {
    async fn new(config: TransportConfig) -> Result<Self> {
        let url = config
            .websocket_url
            .unwrap_or_else(|| format!("ws://{}:{}", config.host, config.port));

        info!("Connecting to WebSocket at {}", url);

        // Create WebSocket
        let ws = WebSocket::new(&url)
            .map_err(|e| SpiceError::Connection(format!("Failed to create WebSocket: {:?}", e)))?;

        // Set binary type
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Add auth header if provided
        if let Some(auth_token) = config.auth_token {
            // Note: WebSocket API doesn't support custom headers directly
            // Auth token should be passed as query parameter or through subprotocol
            info!("Auth token provided (would be used in production)");
        }

        // Create channel for incoming messages
        let (tx, rx) = mpsc::channel(100);
        let is_connected = Rc::new(RefCell::new(false));

        // Set up event handlers
        setup_websocket_handlers(&ws, tx, is_connected.clone()).await?;

        Ok(Self {
            ws,
            rx,
            read_buffer: BytesMut::new(),
            is_connected,
        })
    }
}

#[async_trait(?Send)]
impl Transport for WebSocketTransport {
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Check if we have buffered data
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buf.len(), self.read_buffer.len());
            buf[..len].copy_from_slice(&self.read_buffer[..len]);
            self.read_buffer.advance(len);
            return Ok(len);
        }

        // Wait for new data from WebSocket
        match self.rx.recv().await {
            Some(data) => {
                let len = std::cmp::min(buf.len(), data.len());
                buf[..len].copy_from_slice(&data[..len]);

                // Buffer any remaining data
                if data.len() > len {
                    self.read_buffer.extend_from_slice(&data[len..]);
                }

                Ok(len)
            }
            None => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "WebSocket closed",
            )),
        }
    }

    async fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write_all(buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.ws.ready_state() != WebSocket::OPEN {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "WebSocket not connected",
            ));
        }

        self.ws.send_with_u8_array(buf).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to send: {:?}", e))
        })?;

        Ok(())
    }

    async fn flush(&mut self) -> io::Result<()> {
        // WebSocket sends immediately, no flush needed
        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.is_connected.borrow() && self.ws.ready_state() == WebSocket::OPEN
    }

    async fn close(&mut self) -> io::Result<()> {
        self.ws.close().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to close: {:?}", e))
        })?;
        Ok(())
    }
}

async fn setup_websocket_handlers(
    ws: &WebSocket,
    tx: mpsc::Sender<Vec<u8>>,
    is_connected: Rc<RefCell<bool>>,
) -> Result<()> {
    let ws_clone = ws.clone();

    // Connection opened
    let is_connected_open = is_connected.clone();
    let onopen_callback = Closure::wrap(Box::new(move |_| {
        info!("WebSocket connection opened");
        *is_connected_open.borrow_mut() = true;
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    // Message received
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let data = array.to_vec();

            let tx = tx.clone();
            spawn_local(async move {
                if let Err(e) = tx.send(data).await {
                    error!("Failed to send received data: {}", e);
                }
            });
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Error occurred
    let is_connected_error = is_connected.clone();
    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        error!("WebSocket error: {}", e.message());
        *is_connected_error.borrow_mut() = false;
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // Connection closed
    let is_connected_close = is_connected.clone();
    let onclose_callback = Closure::wrap(Box::new(move |_| {
        info!("WebSocket connection closed");
        *is_connected_close.borrow_mut() = false;
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    // Wait for connection to open
    let timeout = instant::Instant::now() + instant::Duration::from_secs(5);
    while ws_clone.ready_state() == WebSocket::CONNECTING {
        if instant::Instant::now() > timeout {
            return Err(SpiceError::Connection(
                "WebSocket connection timeout".to_string(),
            ));
        }
        crate::utils::sleep(instant::Duration::from_millis(50)).await;
    }

    if ws_clone.ready_state() != WebSocket::OPEN {
        return Err(SpiceError::Connection(
            "WebSocket failed to connect".to_string(),
        ));
    }

    Ok(())
}

/// Create a WebSocket transport
pub async fn create_websocket_transport(config: TransportConfig) -> Result<Box<dyn Transport>> {
    let transport = WebSocketTransport::new(config).await?;
    Ok(Box::new(transport))
}
