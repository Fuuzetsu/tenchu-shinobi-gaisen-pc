use super::{Session, point};
use crate::debug::{Anchor, Debug, Label, overlay::Builder};
use glam::Vec3;
use tenchu_core::{
    math::Position,
    tenchu2::{
        actor::State,
        ai::{
            runtime::Agent,
            sight::{self, Body, Observation, Visibility},
        },
        terrain,
    },
};

fn query(session: &Session, agent: &Agent, stance: u8) -> tenchu_assets::Result<Observation> {
    let world = &session.world;
    let resources = &session.assets.resources;
    let player = &world.actors[world.player].actor;
    let mut target = agent
        .actor
        .target
        .and_then(|id| world.actors.get(id))
        .map(|a| Body::from(&a.actor));
    if let Some(target) = &mut target {
        match stance {
            1 => target.state = State::Normal,
            2 => target.state = State::Crouched,
            _ => {}
        }
    }
    let tick = world.tick.wrapping_add(
        0u32.wrapping_sub(world.tick)
            .wrapping_sub(agent.slot as u32)
            & 31,
    );
    let mut cover = world.ai.cover.clone();
    agent.sight.clone().observe(
        &sight::Observer {
            body: Body::from(&agent.actor),
            kind: agent.actor.kind,
            slot: agent.slot,
            weapon: resources
                .registry
                .actor(agent.actor.kind)
                .ok_or("Missing sight definition")?
                .weapon,
            range: agent.sight_range,
        },
        target.as_ref(),
        &mut sight::Context {
            tick,
            player_position: player.position,
            player_yaw: player.rotation.y,
            script_mode: world.script,
            range_percent: world.ai.sight_percent,
            hidden: world.ai.ignore_player
                && world.script == 0
                && agent.actor.target == Some(world.player),
            terrain: &world.terrain,
            math: &resources.math,
            near_object: &mut || {
                cover
                    .check(&world.collision, player.position, tick as i32)
                    .is_some()
            },
        },
        &mut world.ai.ranges.clone(),
        &mut world.cursor.clone(),
    )
}

pub(super) fn draw(session: &Session, debug: &Debug, drawing: &mut Builder) {
    let world = &session.world;
    let crouched = match debug.sight_stance {
        1 => false,
        2 => true,
        _ => matches!(
            world.actors[world.player].actor.state,
            State::Crouched | State::Wall
        ),
    };
    for (id, agent) in world.actors.iter().enumerate() {
        let actor = &agent.actor;
        if id == world.player || actor.health <= 0 || actor.flags & (8 | 0x10) != 0x10 {
            continue;
        }
        let eye = actor
            .position
            .wrapping_sub(Position::new(0, i32::from(actor.height), 0));
        let range = sight::scale_range(
            i32::from(agent.sight_range) * if crouched { 7 } else { 10 } / 10,
            world.ai.sight_percent,
        );
        let half = if crouched { 349 } else { 699 };
        let points: Vec<_> = (0..=24)
            .map(|sample| {
                let angle = i32::from(actor.rotation.y) - half + 2 * half * sample / 24;
                let [sin, cos] = session.assets.resources.math.pair(angle as i16);
                let delta = Position::new(-sin * range / 4096, 0, -cos * range / 4096);
                point(
                    terrain::segment(
                        &world.terrain,
                        &mut Default::default(),
                        eye,
                        delta,
                        &session.assets.resources.math,
                    )
                    .ok()
                    .flatten()
                    .unwrap_or(eye.wrapping_add(delta)),
                )
            })
            .collect();
        let color =
            if world.ai.ignore_player && world.script == 0 && actor.target == Some(world.player) {
                [0.45, 0.45, 0.45]
            } else {
                [0.4, 0.8, 0.35]
            };
        for pair in points.windows(2) {
            if pair[0].distance(pair[1]) < range as f32 * 0.25 {
                drawing.line(pair[0], pair[1], color);
            }
        }
        drawing.line(point(eye), points[0], color);
        drawing.line(point(eye), points[24], color);
        if let Ok(observation) = query(session, agent, debug.sight_stance) {
            let color = match observation.visibility {
                Visibility::Recognized => [1.0, 0.25, 0.15],
                Visibility::Suspicious => [1.0, 0.8, 0.15],
                _ => [0.4, 0.8, 1.0],
            };
            if let Some(ray) = observation.ray {
                let end = point(ray.obstruction.unwrap_or(ray.target));
                drawing.line(point(ray.eye), end, color);
                if ray.obstruction.is_some() {
                    drawing.box_at(end - Vec3::splat(60.0), end + Vec3::splat(60.0), color);
                }
            }
            if id == debug.selected_actor {
                drawing.labels.push(Label(
                    Anchor::BottomLeft([10.0, 72.0]),
                    format!(
                        "#{id} next sight check: {:?} · {}",
                        observation.visibility, observation.reason
                    ),
                    color,
                ));
            }
        }
    }
    drawing.labels.push(Label(Anchor::BottomLeft([10.0,50.0]),"T2 sight: eye-height bounds; rays stop at native terrain. Queries assume the next perception update.".into(),[0.7,0.95,1.0]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Japanese TENCHU2_DISC; source clear-ray fixture, live range control, overlay and obstruction"]
    fn native_sight_overlay_uses_scaled_ranges_without_mutating_the_world() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut session = Session::load(&disc, 9, 17).unwrap();
        session.world.terrain = tenchu_assets::tenchu2::level::Level::load(&disc, 1)
            .unwrap()
            .collision;
        let player = session.world.player;
        let guard = session
            .world
            .actors
            .iter()
            .position(|a| matches!(a.actor.kind & 0xf0, 0x20..=0x50))
            .unwrap();
        // Clear ray from the recorded JP map-1 perception fixture.
        let actor = &mut session.world.actors[guard].actor;
        actor.kind = 0x20;
        actor.position = [18380, 2000, -20820].into();
        actor.rotation.y = 0;
        actor.height = 500;
        actor.target = Some(player);
        actor.state = State::Ready;
        actor.flags = 0x10;
        session.world.actors[guard].sight_range = 24000;
        let target = &mut session.world.actors[player].actor;
        target.position = [18380, 1500, -29820].into();
        target.height = 2000;
        target.state = State::Normal;
        target.motion.action = 0;
        target.flags &= !0x2000;
        session.world.ai.ranges = sight::Ranges::new(24000);
        let mut baseline_ray = None;
        for (percent, expected) in [
            (100, Visibility::Recognized),
            (50, Visibility::Suspicious),
            (37, Visibility::Unnoticed),
            (25, Visibility::Hidden),
            (0, Visibility::Hidden),
            (200, Visibility::Recognized),
            (100, Visibility::Recognized),
        ] {
            session.world.ai.sight_percent = percent;
            let before = postcard::to_allocvec(&session.world).unwrap();
            let result = query(&session, &session.world.actors[guard], 0).unwrap();
            assert_eq!(result.visibility, expected, "{percent}%: {}", result.reason);
            if let Some(ray) = result.ray {
                let points = (ray.eye, ray.target, ray.obstruction);
                assert_eq!(*baseline_ray.get_or_insert(points), points);
            }
            let mut drawing = Builder::new(glam::Mat4::IDENTITY, 16.0 / 9.0, true);
            let mut debug = Debug::default();
            debug.selected_actor = guard;
            draw(&session, &debug, &mut drawing);
            let overlay = drawing.finish();
            assert!(
                overlay
                    .labels
                    .iter()
                    .any(|label| label.1.contains(&format!("{expected:?}")))
            );
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
        }
        session.world.ai.sight_percent = 50;
        assert_eq!(
            query(&session, &session.world.actors[guard], 2)
                .unwrap()
                .visibility,
            Visibility::Unnoticed
        );
        let actor = &mut session.world.actors[guard].actor;
        actor.position = [-28260, -14500, 20710].into();
        actor.height = 2000;
        let target = &mut session.world.actors[player].actor;
        target.position = [-28260, -14500, 15710].into();
        target.height = 800;
        target.flags |= 0x2000;
        session.world.ai.sight_percent = 400;
        let blocked = query(&session, &session.world.actors[guard], 0).unwrap();
        assert_eq!(blocked.visibility, Visibility::Hidden);
        assert_eq!(blocked.reason, "Terrain blocks sight");
        assert!(blocked.ray.unwrap().obstruction.is_some());
        session.world.ai.ignore_player = true;
        session.world.script = 0;
        let hidden = query(&session, &session.world.actors[guard], 0).unwrap();
        assert_eq!(hidden.visibility, Visibility::Hidden);
        assert_eq!(hidden.reason, "Never noticed enabled");
    }
}
