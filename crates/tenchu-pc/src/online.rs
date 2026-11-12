pub mod artwork;
#[cfg(not(target_arch = "wasm32"))]
mod download;
#[cfg(not(target_arch = "wasm32"))]
mod http;
pub mod ui;
pub mod updates;

const REPOSITORY: &str = "https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc";
