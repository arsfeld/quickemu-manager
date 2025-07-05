//! # spice-client
//!
//! A pure Rust implementation of the SPICE (Simple Protocol for Independent Computing Environments)
//! client protocol with support for both native and WebAssembly targets.
//!
//! ## Features
//!
//! - **Pure Rust** - No C dependencies, easy to build and integrate
//! - **Async/await** - Modern async API using Tokio for native and wasm-bindgen for WASM
//! - **Cross-platform** - Works on Linux, macOS, Windows, and WebAssembly
//! - **WebAssembly Support** - Run SPICE clients directly in web browsers
//! - **Multiple Channels** - Main, Display, Inputs, and Cursor channels implemented
//! - **Authentication** - RSA-OAEP (Spice ticket) authentication support
//! - **Multi-display** - Support for multiple display surfaces
//! - **Extensible** - Easy to add new channel types
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! spice-client = "0.1.0"
//!
//! # For native builds
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ## Basic Example
//!
//! ```no_run
//! use spice_client::{SpiceClient, SpiceError};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), SpiceError> {
//!     // Create a new SPICE client
//!     let mut client = SpiceClient::new("localhost", 5900);
//!     
//!     // Connect to the SPICE server
//!     client.connect().await?;
//!     
//!     // The client handles messages internally through channels
//!     // You can interact with display, input, and other channels
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## WebAssembly Example
//!
//! ```ignore
//! use spice_client::{SpiceClient, SpiceError};
//! use wasm_bindgen_futures::spawn_local;
//!
//! fn connect_to_spice() {
//!     spawn_local(async {
//!         // WebSocket proxy URL (ws:// or wss://)
//!         let mut client = SpiceClient::new(
//!             "ws://localhost:8080/spice".to_string(),
//!             0  // Port is included in WebSocket URL
//!         );
//!         
//!         if let Err(e) = client.connect().await {
//!             web_sys::console::error_1(&format!("Connection failed: {:?}", e).into());
//!         }
//!     });
//! }
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - **`protocol`** - SPICE protocol message definitions and serialization
//! - **`client`** - Native client implementation using Tokio
//! - **`wasm_bindings`** - WebAssembly client using browser APIs
//! - **`channels`** - Individual channel implementations (Main, Display, Inputs, Cursor)
//! - **`error`** - Error types and result definitions
//!
//! ## Supported Channels
//!
//! - **Main Channel** - Connection setup, mouse modes, agent communication
//! - **Display Channel** - Screen updates, drawing commands, video streaming
//! - **Inputs Channel** - Keyboard and mouse input
//! - **Cursor Channel** - Hardware cursor updates
//!
//! ## Current Limitations
//!
//! This library is functional but still under active development. Current limitations include:
//!
//! - **No audio support** - Playback and Record channels not implemented
//! - **No USB redirection** - USB channel not implemented  
//! - **No clipboard sharing** - Agent clipboard integration not implemented
//! - **Limited compression** - Only ZLIB compression supported (no LZ4)
//! - **No TLS encryption** - Only unencrypted connections supported
//! - **WebSocket proxy required for WASM** - Cannot connect directly to SPICE TCP ports from browsers
//! - **Partial drawing commands** - Some complex QXL drawing operations not fully implemented
//!
//! ## Platform-Specific Considerations
//!
//! ### Native Builds
//! - Full Tokio runtime required
//! - Direct TCP connection to SPICE servers
//! - Multi-threaded event processing
//!
//! ### WebAssembly Builds  
//! - Requires WebSocket proxy (see examples/websocket-proxy.py)
//! - Single-threaded event loop
//! - Cannot create multiple TCP connections per WebSocket
//! - Some browser security restrictions apply
//!
//! ## Contributing
//!
//! Contributions are welcome! Please check the [GitHub repository](https://github.com/yourusername/spice-client)
//! for contribution guidelines.
//!
//! ## License
//!
//! This project is licensed under the MIT License - see the LICENSE file for details.

// #![warn(missing_docs)]  // TODO: Add documentation for all public items
#![warn(rustdoc::missing_crate_level_docs)]

pub mod channels;
pub mod client;
pub mod client_shared;
pub mod error;
pub mod protocol;
pub mod transport;
pub mod utils;
pub mod video;
pub mod wire_format;

#[cfg(not(target_arch = "wasm32"))]
pub mod multimedia;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub mod wasm_bindings;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// For non-WASM builds, export the native client
#[cfg(not(target_arch = "wasm32"))]
pub use client::SpiceClient;

// For WASM builds, export the WASM-specific client
#[cfg(target_arch = "wasm32")]
pub use wasm_bindings::SpiceClient;

// Export types for convenience
pub type Client = SpiceClient;

/// Builder for creating SPICE clients
pub struct ClientBuilder {
    host: String,
    port: u16,
    password: Option<String>,
}

impl ClientBuilder {
    /// Create a new client builder from a URI
    pub fn new(uri: &str) -> Self {
        // Parse URI to extract host and port
        let uri = uri.trim_start_matches("spice://");
        let parts: Vec<&str> = uri.split(':').collect();
        let host = parts.first().unwrap_or(&"localhost").to_string();
        let port = parts
            .get(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(5900);

        Self {
            host,
            port,
            password: None,
        }
    }

    /// Set the password for authentication
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    /// Build the client
    pub fn build(self) -> Result<SpiceClient> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut client = SpiceClient::new(self.host, self.port);
            if let Some(password) = self.password {
                client.set_password(password);
            }
            Ok(client)
        }
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, we can't create a client without a canvas element
            // This builder pattern doesn't work well for WASM
            Err(SpiceError::Connection(
                "Cannot create WASM client without canvas element. Use SpiceClient::new() directly.".to_string()
            ))
        }
    }
}

pub use client_shared::SpiceClientShared;
pub use error::{Result, SpiceError};
pub use protocol::*;
pub use video::{VideoFrame, VideoOutput};

// Re-export commonly used types
pub use channels::{DisplaySurface, InputEvent, KeyCode, MouseButton};
