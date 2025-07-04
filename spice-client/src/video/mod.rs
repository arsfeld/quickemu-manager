mod frame;
mod output;

pub use frame::VideoFrame;
pub use output::{create_video_output, VideoOutput};

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod wasm;
