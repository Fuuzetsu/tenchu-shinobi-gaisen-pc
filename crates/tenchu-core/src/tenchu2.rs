pub mod actor;
pub mod ai;
pub mod audio;
pub mod camera;
pub mod campaign;
pub mod collision;
pub mod control;
pub mod effects;
pub mod environment;
pub mod hud;
pub mod input;
pub mod introduction;
pub mod inventory;
pub mod items;
pub mod mission;
pub mod motion;
pub mod physics;
pub mod progress;
pub mod rig;
pub mod story;
pub mod terrain;
pub mod visibility;
pub mod world;

pub fn horizontal_distance(
    a: crate::math::Position,
    b: crate::math::Position,
    math: &crate::math::Math,
) -> i32 {
    let x = a.x.wrapping_sub(b.x);
    let z = a.z.wrapping_sub(b.z);
    if x.wrapping_abs() >= 32767 || z.wrapping_abs() >= 32767 {
        32767
    } else {
        math.square_root(x.wrapping_mul(x).wrapping_add(z.wrapping_mul(z)))
    }
}
