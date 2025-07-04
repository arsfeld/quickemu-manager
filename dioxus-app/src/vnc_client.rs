use js_sys::{ArrayBuffer, Uint8Array};
use std::sync::Arc;
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

#[derive(Debug, Clone)]
pub enum VncMessage {
    Connect(String, Option<String>), // URL, optional auth token
    Disconnect,
    SendData(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum VncEvent {
    Connected,
    Disconnected,
    Error(String),
    Data(Vec<u8>),
}

pub struct VncClient {
    websocket: Option<WebSocket>,
    event_tx: mpsc::UnboundedSender<VncEvent>,
    message_rx: mpsc::UnboundedReceiver<VncMessage>,
    onopen: Option<Closure<dyn FnMut(JsValue)>>,
    onmessage: Option<Closure<dyn FnMut(MessageEvent)>>,
    onerror: Option<Closure<dyn FnMut(ErrorEvent)>>,
    onclose: Option<Closure<dyn FnMut(CloseEvent)>>,
}

impl VncClient {
    pub fn new() -> (
        Self,
        mpsc::UnboundedReceiver<VncEvent>,
        mpsc::UnboundedSender<VncMessage>,
    ) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let client = Self {
            websocket: None,
            event_tx,
            message_rx,
            onopen: None,
            onmessage: None,
            onerror: None,
            onclose: None,
        };

        (client, event_rx, message_tx)
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                VncMessage::Connect(url, auth_token) => {
                    if let Err(e) = self.connect(&url, auth_token).await {
                        let _ = self
                            .event_tx
                            .send(VncEvent::Error(format!("Connection failed: {:?}", e)));
                    }
                }
                VncMessage::Disconnect => {
                    self.disconnect();
                }
                VncMessage::SendData(data) => {
                    if let Err(e) = self.send_data(&data) {
                        let _ = self
                            .event_tx
                            .send(VncEvent::Error(format!("Send failed: {:?}", e)));
                    }
                }
            }
        }
    }

    async fn connect(&mut self, url: &str, auth_token: Option<String>) -> Result<(), JsValue> {
        log::info!("VncClient: Connecting to WebSocket URL: {}", url);

        // Create WebSocket
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        log::debug!("VncClient: WebSocket created successfully");

        let event_tx = self.event_tx.clone();
        let auth_token_clone = auth_token.clone();

        // Set up onopen handler
        let ws_clone = ws.clone();
        let event_tx_open = event_tx.clone();
        let onopen = Closure::wrap(Box::new(move |_| {
            log::info!("VncClient: WebSocket opened");

            // Send auth token if provided
            if let Some(token) = &auth_token_clone {
                log::debug!("VncClient: Sending auth token");
                match ws_clone.send_with_str(token) {
                    Ok(_) => log::debug!("VncClient: Auth token sent successfully"),
                    Err(e) => log::error!("VncClient: Failed to send auth token: {:?}", e),
                }
            }
            let _ = event_tx_open.send(VncEvent::Connected);
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        self.onopen = Some(onopen);

        // Set up onmessage handler
        let event_tx_msg = event_tx.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(arraybuffer) = e.data().dyn_into::<ArrayBuffer>() {
                let array = Uint8Array::new(&arraybuffer);
                let data = array.to_vec();
                log::debug!("VncClient: Received binary message, {} bytes", data.len());
                let _ = event_tx_msg.send(VncEvent::Data(data));
            } else if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                // Handle text messages (like "authenticated")
                let msg: String = text.into();
                log::debug!("VncClient: Received text message: {}", msg);
                if msg == "authenticated" {
                    log::info!("VNC authentication successful");
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        self.onmessage = Some(onmessage);

        // Set up onerror handler
        let event_tx_err = event_tx.clone();
        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("VncClient: WebSocket error: {}", e.message());
            let _ = event_tx_err.send(VncEvent::Error(format!(
                "WebSocket error: {:?}",
                e.message()
            )));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        self.onerror = Some(onerror);

        // Set up onclose handler
        let event_tx_close = event_tx.clone();
        let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
            log::info!(
                "VncClient: WebSocket closed: code={}, reason={}, wasClean={}",
                e.code(),
                e.reason(),
                e.was_clean()
            );
            let _ = event_tx_close.send(VncEvent::Disconnected);
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        self.onclose = Some(onclose);

        self.websocket = Some(ws);
        Ok(())
    }

    fn disconnect(&mut self) {
        log::info!("VncClient: Disconnecting WebSocket");

        if let Some(ws) = &self.websocket {
            let _ = ws.close();
        }
        self.websocket = None;

        // Clear closures
        self.onopen = None;
        self.onmessage = None;
        self.onerror = None;
        self.onclose = None;

        let _ = self.event_tx.send(VncEvent::Disconnected);
    }

    fn send_data(&self, data: &[u8]) -> Result<(), JsValue> {
        log::debug!("VncClient: Sending {} bytes of data", data.len());
        if let Some(ws) = &self.websocket {
            if ws.ready_state() == WebSocket::OPEN {
                let array = Uint8Array::from(data);
                ws.send_with_array_buffer(&array.buffer())
            } else {
                Err(JsValue::from_str("WebSocket not open"))
            }
        } else {
            Err(JsValue::from_str("WebSocket not connected"))
        }
    }
}
