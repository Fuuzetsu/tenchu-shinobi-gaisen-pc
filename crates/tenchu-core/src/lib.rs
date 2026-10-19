pub mod actor;
pub mod camera;
pub mod clock;
pub mod collision;
pub mod effects;
pub mod game;
pub mod ids;
pub mod input;
pub mod math;
pub mod motion;
pub mod random;
pub mod scene;
pub mod terrain;

pub use tenchu_assets::Result;

pub const TICKS_PER_SECOND: u32 = 30;
