mod audio;
mod campaign;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
mod controls;
mod debug;
mod demo;
mod diagnostics;
mod fonts;
mod frontend;
mod lighting;
mod navigation;
mod randomizer;
mod recording;
mod render_effects;
mod render_mesh;
mod renderer;
#[cfg(not(target_arch = "wasm32"))]
mod retail;
mod session;
mod settings;
mod source;
#[cfg(not(target_arch = "wasm32"))]
mod startup;
mod storage;
mod ui;
#[cfg(not(target_arch = "wasm32"))]
mod video;
mod visuals;
mod window;

#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
#[cfg(target_arch = "wasm32")]
mod web;

mod web_assets;
#[cfg(not(target_arch = "wasm32"))]
mod web_export;
