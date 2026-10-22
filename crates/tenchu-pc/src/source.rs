#[cfg(not(target_arch = "wasm32"))]
#[path = "source_native.rs"]
mod platform;
#[cfg(target_arch = "wasm32")]
#[path = "web/source.rs"]
mod platform;
pub use platform::*;
