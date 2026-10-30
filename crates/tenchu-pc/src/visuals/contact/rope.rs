use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
};
use bytemuck::Zeroable;
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game};

pub fn geometry(game: &Game) -> Geometry {
    let mut vertices = Vec::new();
    let point = |p: tenchu_core::math::Position| Vec3::from(p.map(|n| n as f32).to_array());
    let flying = game.effects.active_slots().iter().flatten().find_map(|e| {
        if let Effect::FlyWire(w) = e {
            Some(w)
        } else {
            None
        }
    });
    let path = if let Some(wire) = flying {
        let start = point(wire.start);
        let end = point(wire.end);
        let center = if wire.mode == 0 {
            point(wire.center)
        } else {
            point(wire.center).lerp(
                point(wire.straight),
                (f32::from(wire.count.saturating_sub(1)) / 5.0).clamp(0.0, 1.0),
            )
        };
        Some((
            start,
            center,
            end,
            if wire.mode == 0 {
                f32::from(wire.count).clamp(0.0, 4096.0) / 4096.0
            } else {
                1.0
            },
        ))
    } else if !game.effects.wires.is_empty()
        && game.player().motion.action == tenchu_core::ids::MOT_KAGI_PULL
    {
        let actor = game.player();
        let start = point(actor.rendered[14].translation);
        let end = point(game.camera.target);
        Some((
            start,
            (start + end) * 0.5 + Vec3::Y * start.distance(end) / 32.0,
            end,
            1.0,
        ))
    } else {
        None
    };
    if let Some((start, center, end, length)) = path {
        let count = ((start.distance(end) * length / 180.0).ceil() as usize).clamp(2, 96);
        let curve = |t: f32| start * (1.0 - t).powi(2) + center * 2.0 * t * (1.0 - t) + end * t * t;
        let correction = game
            .effects
            .wires
            .iter()
            .find_map(|w| w.hook)
            .map_or(Vec3::ZERO, |(hook, _)| {
                point(hook.translation) - curve(length)
            });
        let curve = |t: f32| curve(t * length) + correction * t;
        for segment in 0..count {
            let a = curve(segment as f32 / count as f32);
            let b = curve((segment + 1) as f32 / count as f32);
            let direction = (b - a).normalize_or_zero();
            if direction == Vec3::ZERO {
                continue;
            }
            let right = direction.any_orthonormal_vector();
            let up = direction.cross(right);
            for side in 0..8 {
                let quad = std::array::from_fn::<_, 4, _>(|i| {
                    let angle = (side + i % 2) as f32 * std::f32::consts::TAU / 8.0;
                    let normal = right * angle.cos() + up * angle.sin();
                    let weave = 0.8 + 0.2 * (angle * 3.0 - (segment + i / 2) as f32 * 2.5).sin();
                    Vertex {
                        position: ((if i / 2 == 0 { a } else { b }) + normal * 16.0).to_array(),
                        normal: normal.to_array(),
                        color: [0.38 * weave, 0.30 * weave, 0.19 * weave],
                        opacity: 1.0,
                        flags: render_mesh::FOG | render_mesh::DOUBLE_SIDED,
                        ..Vertex::zeroed()
                    }
                });
                vertices.extend([0, 1, 2, 1, 3, 2].map(|i| quad[i]));
            }
        }
    }
    let count = vertices.len() as u32;
    Geometry {
        vertices,
        batches: vec![Batch {
            blend: 0,
            range: 0..count,
        }],
    }
}
