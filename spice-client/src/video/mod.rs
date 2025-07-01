mod frame;
mod output;

pub use frame::VideoFrame;
pub use output::{VideoOutput, create_video_output};

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod wasm;