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
        let range = i32::from(agent.sight_range) * if crouched { 7 } else { 10 } / 10;
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
        let color = [0.4, 0.8, 0.35];
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
