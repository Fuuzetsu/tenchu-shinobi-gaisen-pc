use crate::debug::{Anchor, Debug, Label, Overlay, overlay::Builder};
use glam::Vec3;
use tenchu_core::{actor::State as ActorState, game::Game, ids::ATTR_SUSPEND};

pub fn build(debug: &Debug, game: &Game, aspect: f32) -> Overlay {
    let mut lines = Builder::new(debug.matrix(game, aspect), aspect, debug.xray);
    if let Some((message, at)) = &debug.notice
        && at.elapsed().as_secs() < 5
    {
        lines.labels.push(Label(
            Anchor::BottomLeft([10.0, 72.0]),
            message.clone(),
            [0.9, 0.95, 1.0],
        ));
    }
    if debug.collision || debug.collision_only {
        for surface in &debug.surfaces {
            lines.surface(surface.corners, surface.color);
        }
    }
    if debug.hitboxes {
        for slot in &game.collision.slots[..game.collision.len] {
            let Some(model) = slot.model else {
                continue;
            };
            if game.scene.models[model].attribute & tenchu_core::scene::COLLIDE == 0 {
                continue;
            }
            if slot
                .actor()
                .and_then(|id| game.roster.actors.get(id))
                .and_then(Option::as_ref)
                .is_some_and(|a| a.attribute & ATTR_SUSPEND != 0)
            {
                continue;
            }
            let center = Vec3::from(slot.position.map(|v| v as f32).to_array());
            let size = Vec3::from(slot.size.map(f32::from));
            lines.collider(center, size, slot.class & tenchu_core::collision::HIT != 0);
        }
    }
    for &id in &game.roster.order {
        let actor = game.roster.get(id);
        let position = Vec3::from(actor.position.map(|v| v as f32).to_array());
        let suspended = actor.attribute & ATTR_SUSPEND != 0;
        if debug.labels {
            lines.actor(super::actor(game, actor), debug.selected_actor);
        }
        if debug.patrols
            && let Some(patrol) = &actor.patrol
        {
            let points: Vec<_> = patrol
                .points
                .iter()
                .filter(|p| p.pad != 0xffff)
                .map(|p| {
                    let mut cursor = tenchu_core::terrain::Cursor::new(&game.mission.terrain);
                    let p = [p.position[0], actor.position[1] - 1500, p.position[1]];
                    let floor = tenchu_core::terrain::level(
                        &game.mission.terrain,
                        &mut cursor,
                        p.into(),
                        0,
                    )
                    .ok()
                    .filter(|&v| v != tenchu_core::terrain::NONE)
                    .unwrap_or(actor.position[1]);
                    Vec3::new(p[0] as f32, floor as f32 - 30.0, p[2] as f32)
                })
                .collect();
            if points.len() > 1 {
                for i in 0..points.len() {
                    lines.line(points[i], points[(i + 1) % points.len()], [0.95, 0.55, 0.9]);
                }
            }
        }
        if debug.sight
            && id != game.player_id
            && !suspended
            && actor.health > 0
            && actor.attribute & tenchu_core::ids::ATTR_CUSTOMAI != 0
        {
            let sneaking = match debug.sight_stance {
                1 => false,
                2 => true,
                _ => matches!(game.player().status, ActorState::Crouch | ActorState::Wall),
            };
            let occluders = debug.sight_occluders.get_or_init(|| {
                crate::debug::sight::Occluders::new(&crate::render_mesh::stage(
                    &game.mission,
                    &game.math,
                    None,
                ))
            });
            let profile = tenchu_core::actor::sight::profile(
                sneaking,
                game.player().sight_percent(debug.sight_percent),
            );
            let eye = position
                + Vec3::Y * (300.0 - f32::from(game.mission.characters[actor.definition].height));
            for (range, color) in [
                (profile.clear, [0.35, 1.0, 0.3]),
                (profile.sight, [1.0, 0.75, 0.2]),
            ] {
                for dy in [-2999.0f32, 0.0, 2999.0] {
                    if range as f32 <= dy.abs() {
                        continue;
                    }
                    let radius = ((range as f32).powi(2) - dy * dy).sqrt();
                    let points: Vec<_> = (0..=64)
                        .map(|i| {
                            let angle = (f32::from(actor.rotation[1])
                                - f32::from(profile.half_angle)
                                + 2.0 * f32::from(profile.half_angle) * i as f32 / 64.0)
                                * std::f32::consts::TAU
                                / 4096.0;
                            let target =
                                eye + Vec3::new(-angle.sin() * radius, dy, -angle.cos() * radius);
                            occluders.endpoint(eye, target)
                        })
                        .collect();
                    for pair in points.windows(2) {
                        if pair[0].distance(pair[1]) < radius * 0.15 {
                            lines.line(pair[0], pair[1], color);
                        }
                    }
                    lines.line(eye, points[0], color);
                    lines.line(eye, points[64], color);
                }
            }
        }
    }
    if debug.events {
        for event in &game.mission.scripts.events {
            let bounds = match event.mode {
                1 => Some(event.bounds.map(|[lo, hi]| {
                    [
                        i32::from(lo) * 1000 - if lo <= 0 { 999 } else { 0 },
                        i32::from(hi) * 1000 + if hi >= 0 { 999 } else { 0 },
                    ]
                })),
                6 => {
                    let id = if event.target == 255 {
                        Some(game.player_id)
                    } else {
                        game.roster.find(i16::from(event.target))
                    };
                    id.map(|id| {
                        game.roster
                            .get(id)
                            .position
                            .map(|v| [v - 2000, v + 2000])
                            .to_array()
                    })
                }
                _ => None,
            };
            if let Some(b) = bounds {
                let min = Vec3::new(b[0][0] as f32, b[1][0] as f32, b[2][0] as f32);
                let max = Vec3::new(b[0][1] as f32, b[1][1] as f32, b[2][1] as f32);
                lines.box_at(min, max, [0.9, 0.35, 0.95]);
                if let Some([x, y]) = lines.project((min + max) * 0.5) {
                    lines.labels.push(Label(
                        Anchor::Projected([x, y]),
                        format!("Event {} CVA {}", event.id, event.sequence),
                        [0.9, 0.35, 0.95],
                    ));
                }
            }
        }
    }
    if debug.show_trail {
        for (a, b) in debug.trail.iter().zip(debug.trail.iter().skip(1)) {
            if a.distance_squared(*b) < 4000.0f32.powi(2) {
                lines.line(*a - Vec3::Y * 20.0, *b - Vec3::Y * 20.0, [1.0, 0.85, 0.2]);
            }
        }
    }
    if debug.inspect_actor
        && let Some(actor) = game
            .roster
            .actors
            .get(debug.selected_actor)
            .and_then(Option::as_ref)
    {
        lines.inspection([
            format!(
                "#{} kind {:02x} HP {}/{}",
                actor.id, actor.kind, actor.health, actor.max_health
            ),
            format!(
                "XYZ {} {} {}",
                actor.position[0], actor.position[1], actor.position[2]
            ),
            format!(
                "Yaw {} motion {:04x}:{}",
                actor.rotation[1], actor.motion.action, actor.motion.count
            ),
            format!(
                "State {:?} flags {:04x} AI {:?}",
                actor.status, actor.attribute, actor.think
            ),
        ]);
    }
    if let Some(subject) = debug.camera_actor {
        let status = if debug
            .followed_actor(game)
            .is_some_and(|actor| actor.attribute & tenchu_core::ids::ATTR_SUSPEND != 0)
        {
            " (suspended)"
        } else {
            ""
        };
        lines.labels.push(Label(
            Anchor::BottomLeft([10.0, 24.0]),
            format!(
                "FOLLOW #{}{status}  RMB/right stick: orbit  Middle click/R3: recenter  F2: return",
                subject.id
            ),
            [0.7, 0.95, 1.0],
        ));
    }
    if debug.camera.is_some() {
        lines.labels.push(Label(
            Anchor::BottomLeft([10.0, 48.0]),
            if cfg!(target_os = "android") {
                "FREE CAMERA  Move / Camera sticks"
            } else {
                "FREE CAMERA  WASD/QE  Shift: boost"
            }
            .into(),
            [0.7, 0.95, 1.0],
        ));
        lines.labels.push(Label(
            Anchor::BottomLeft([10.0, 24.0]),
            if debug.pick_actor_position {
                "Click collision floor to choose an actor placement"
            } else if debug.teleport_armed || debug.click_teleport {
                "Click collision floor to teleport"
            } else if cfg!(target_os = "android") {
                "Rise / Descend / Boost · Tap: actor · Back: return"
            } else {
                "RMB: look  Click: actor  F2: return"
            }
            .into(),
            [0.7, 0.95, 1.0],
        ));
    }
    if debug.click_teleport && debug.camera.is_none() {
        lines.labels.push(Label(
            Anchor::BottomLeft([10.0, 24.0]),
            "TELEPORT  Click a floor · Esc: cancel".into(),
            [0.7, 0.95, 1.0],
        ));
    }
    lines.finish()
}
