//! # spice-client
//!
//! A pure Rust implementation of the SPICE (Simple Protocol for Independent Computing Environments) 
//! client protocol with support for both native and WebAssembly targets.
//!
//! ## Features
//!
//! - Pure Rust implementation with no C dependencies
//! - Async/await support using Tokio
//! - WebAssembly support for browser-based clients
//! - Multiple channel support (Main, Display, Inputs, Cursor)
//! - Authentication support
//! - Extensible architecture for adding new channels
//!
//! ## Example
//!
//! ```no_run
//! use spice_client::{SpiceClient, SpiceError};
//!
//! # async fn example() -> Result<(), SpiceError> {
//! let mut client = SpiceClient::new("localhost".to_string(), 5900);
//! client.connect().await?;
//! client.start_event_loop().await?;
//! 
//! // Get display surface when available
//! if let Some(surface) = client.get_display_surface(0).await {
//!     println!("Display size: {}x{}", surface.width, surface.height);
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod protocol;
pub mod client;
pub mod client_shared;
pub mod channels;
pub mod error;
pub mod video;

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

pub use client_shared::SpiceClientShared;
pub use error::{SpiceError, Result};
pub use protocol::*;
pub use video::{VideoFrame, VideoOutput};

// Re-export commonly used types
pub use channels::{DisplaySurface, InputEvent, MouseButton, KeyCode};