pub mod actor;
pub mod ai;
pub mod camera;
pub mod clock;
pub mod collision;
pub mod construction;
pub mod drawing;
pub mod effects;
pub mod game;
pub mod hud;
pub mod ids;
pub mod input;
pub mod items;
pub mod math;
pub mod motion;
pub mod packets;
pub mod projection;
pub mod random;
pub mod roster;
pub mod scene;
pub mod terrain;
pub mod visibility;

#[cfg(test)]
mod oracle;

pub use tenchu_assets::Result;

pub const TICKS_PER_SECOND: u32 = 30;
