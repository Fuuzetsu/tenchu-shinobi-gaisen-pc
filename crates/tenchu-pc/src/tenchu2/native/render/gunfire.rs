use super::super::contact::{GUNSHOT_TICKS, History};
#[cfg(test)]
mod tests;
use crate::{lighting, render_effects};
use glam::{Mat3, Mat4, Vec3};

pub(super) fn geometry(
    history: &History,
    tick: u64,
    camera: Mat4,
    alpha: f32,
) -> render_effects::Geometry {
    let inverse = camera.inverse();
    let near = inverse.project_point3(Vec3::ZERO);
    let forward = (inverse.project_point3(Vec3::Z) - near).normalize_or_zero();
    let right = forward.cross(Vec3::NEG_Y).normalize_or_zero();
    let up = right.cross(forward);
    let mut surfaces = Vec::new();
    for (at, shot) in &history.gunshots {
        let Some(elapsed) = tick.checked_sub(*at).filter(|&age| age < GUNSHOT_TICKS) else {
            continue;
        };
        let center = Vec3::from_array(shot.position.to_array().map(|v| v as f32));
        let depth = forward.dot(center - near);
        if depth <= 0.0 {
            continue;
        }
        render_effects::gun_impact(
            Mat3::from_cols(right, up, forward),
            center,
            Vec3::from_array(shot.direction.to_array().map(|v| v as f32)).normalize_or_zero(),
            shot.part,
            elapsed as f32 + alpha.clamp(0.0, 1.0),
            |position, axes, color, glow, rounded, opacity| {
                if opacity <= 0.0 {
                    return;
                }
                surfaces.push(render_effects::Surface {
                    depth,
                    blend: 5,
                    vertices: render_effects::soft_patch(
                        position, axes, color, glow, rounded, opacity,
                    ),
                });
            },
        );
    }
    render_effects::surface_geometry(surfaces)
}

pub(super) fn lights(history: &History, tick: u64, alpha: f32) -> Vec<lighting::Source> {
    history
        .gunshots
        .iter()
        .filter_map(|(at, shot)| {
            if !matches!(shot.part, tenchu_core::effects::GunPart::Muzzle) {
                return None;
            }
            let elapsed = tick.checked_sub(*at).filter(|&age| age < 3)?;
            lighting::gun_muzzle(
                Vec3::from_array(shot.position.to_array().map(|v| v as f32)),
                elapsed as f32 + alpha.clamp(0.0, 1.0),
            )
        })
        .take(2)
        .collect()
}
