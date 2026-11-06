#[cfg(target_os = "android")]
mod android;
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
mod rewind;
mod savestate;
mod session;
mod settings;
mod sheathing;
mod source;
#[cfg(not(target_arch = "wasm32"))]
mod startup;
mod storage;
mod touch;
mod trial;
mod ui;
#[cfg(not(target_arch = "wasm32"))]
mod video;
mod visuals;
mod walking;
mod window;

#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
#[cfg(target_arch = "wasm32")]
mod web;

mod web_assets;
#[cfg(not(target_arch = "wasm32"))]
mod web_export;
