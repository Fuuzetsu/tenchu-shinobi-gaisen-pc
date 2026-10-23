pub mod appearance;
pub mod catalog;
#[cfg(not(target_arch = "wasm32"))]
mod export;
pub mod interpolation;
pub mod lights;
pub mod models;
pub mod obj;
pub mod pack;
pub mod pixels;
pub mod relief;
mod smooth;

#[cfg(not(target_arch = "wasm32"))]
pub use export::export;
#[cfg(not(target_arch = "wasm32"))]
pub use export::interface::export as export_interface;
