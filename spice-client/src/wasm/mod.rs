//! WebAssembly-specific implementations for SPICE client

#[cfg(target_arch = "wasm32")]
pub mod video_renderer;

#[cfg(target_arch = "wasm32")]
pub mod canvas;

#[cfg(target_arch = "wasm32")]
pub mod cursor;