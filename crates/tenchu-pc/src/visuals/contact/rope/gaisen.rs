use crate::render_effects::Geometry;
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game};

pub fn geometry(game: &Game) -> Geometry {
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
        let points: Vec<_> = (0..=count)
            .map(|segment| curve(segment as f32 / count as f32))
            .collect();
        super::path(&points)
    } else {
        super::path(&[])
    }
}
