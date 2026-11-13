use super::Session;
use crate::debug::{Debug, Overlay, inspection::Actor, overlay::Builder};
use glam::{Mat4, Vec3};
use tenchu_core::{
    math::Position,
    tenchu2::{collision, terrain},
};

mod sight;
pub(super) mod sounds;
mod surfaces;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use surfaces::pick;

fn point(position: Position) -> Vec3 {
    Vec3::from(position.to_array().map(|v| v as f32))
}

pub(super) fn actors(session: &Session) -> impl Iterator<Item = Actor> + '_ {
    session.world.actors.iter().enumerate().map(|(id, agent)| {
        let actor = &agent.actor;
        Actor {
            id,
            root: session.world.instances[id].rig.root,
            yaw: actor.rotation.y,
            position: Vec3::from(actor.position.to_array().map(|v| v as f32)),
            kind: actor.kind,
            health: actor.health,
            height: f32::from(actor.height),
            suspended: actor.flags & 8 != 0,
        }
    })
}

pub(super) fn overlay(session: &Session, debug: &Debug, matrix: Mat4, aspect: f32) -> Overlay {
    let mut drawing = Builder::new(matrix, aspect, debug.xray);
    if debug.photo_mode {
        return drawing.finish();
    }
    debug.draw_trail(&mut drawing);
    if debug.collision || debug.collision_only || debug.events || debug.sound_triggers {
        for (node, corners) in surfaces::surfaces(&session.world.terrain) {
            let trigger = node.attributes & 0xf00 == 0x400;
            let sound = trigger
                && debug.sound_triggers
                && session
                    .world
                    .environment
                    .triggers
                    .get(usize::from(node.attributes & 255))
                    .is_some_and(|trigger| sounds::effect(trigger.kind).is_some());
            if debug.collision || debug.collision_only || (trigger && debug.events) || sound {
                let color = match node.attributes & 0xf00 {
                    0x100 => [0.74, 0.35, 1.0],
                    0x200 | 0x300 => [0.12, 0.57, 1.0],
                    0x400 if sound => [0.15, 0.85, 1.0],
                    0x400 => [1.0, 0.3, 0.15],
                    _ => [0.23, 0.84, 0.65],
                };
                drawing.surface(corners, color);
            }
        }
    }
    if debug.patrols {
        for agent in &session.world.actors {
            let route: Vec<_> = agent
                .behavior
                .patrol
                .waypoints()
                .map(|[x, z]| {
                    let position = Position::new(x, agent.actor.position.y - 1500, z);
                    let y = terrain::level(
                        &session.world.terrain,
                        &mut Default::default(),
                        position,
                        terrain::Query::Surface,
                    )
                    .ok()
                    .flatten()
                    .unwrap_or(agent.actor.position.y);
                    Vec3::new(x as f32, y as f32 - 30.0, z as f32)
                })
                .collect();
            for index in 0..route.len() {
                drawing.line(
                    route[index],
                    route[(index + 1) % route.len()],
                    [0.95, 0.55, 0.9],
                );
            }
            if let Some(&next) = route.get(agent.behavior.patrol.next) {
                drawing.line(point(agent.actor.position), next, [1.0, 0.85, 0.2]);
            }
        }
    }
    if debug.events {
        events(session, &mut drawing);
    }
    if debug.sound_triggers {
        sounds::draw(session, &mut drawing);
    }
    if debug.sight {
        sight::draw(session, debug, &mut drawing);
    }
    if debug.labels {
        for actor in actors(session) {
            drawing.actor(actor, debug.selected_actor);
        }
    }
    if debug.hitboxes {
        for slot in session.world.collision.slots() {
            if slot.flags & (collision::REMOVED | collision::DISABLED) == 0 {
                drawing.collider(
                    Vec3::from(slot.position.to_array().map(|v| v as f32)),
                    Vec3::from(slot.size.to_array().map(f32::from)),
                    slot.flags & collision::WEAPON != 0,
                );
            }
        }
    }
    if debug.inspect_actor
        && let Some(agent) = session.world.actors.get(debug.selected_actor)
    {
        let actor = &agent.actor;
        let ai = &agent.behavior.programs;
        drawing.inspection([
            format!(
                "#{} kind {:02x} HP {}/{}",
                debug.selected_actor, actor.kind, actor.health, actor.max_health
            ),
            format!(
                "XYZ {} {} {}",
                actor.position.x, actor.position.y, actor.position.z
            ),
            format!(
                "Yaw {} motion {:04x}:{}",
                actor.rotation.y, actor.motion.action, actor.motion.frame
            ),
            format!("State {:?} flags {:04x}", actor.state, actor.flags),
            format!("AI idle {:?}", ai.idle),
            format!("AI suspicious {:?}", ai.suspicious),
            format!("AI combat {:?}", ai.combat),
            format!("AI searching {:?}", ai.searching),
        ]);
    }
    drawing.finish()
}

fn events(session: &Session, drawing: &mut Builder) {
    use crate::debug::{Anchor, Label};
    use tenchu_assets::tenchu2::campaign::Objective;
    if let Objective::Reach { region, .. } = session.world.mission.objective {
        // Mission tests divide coordinates by 1000, truncating toward zero.
        let bounds = std::array::from_fn::<_, 3, _>(|axis| {
            let center = i32::from(region.center[axis]);
            let radius = if axis == 1 {
                2
            } else {
                i32::from(region.radius)
            };
            let (lo, hi) = (center - radius, center + radius);
            [
                (lo * 1000 - if lo <= 0 { 999 } else { 0 }) as f32,
                (hi * 1000 + if hi >= 0 { 999 } else { 0 }) as f32,
            ]
        });
        let min = Vec3::from(bounds.map(|v| v[0]));
        let max = Vec3::from(bounds.map(|v| v[1]));
        drawing.box_at(min, max, [0.95, 0.35, 1.0]);
        if let Some(p) = drawing.project((min + max) * 0.5) {
            drawing.labels.push(Label(
                Anchor::Projected(p),
                "Mission destination".into(),
                [0.95, 0.35, 1.0],
            ));
        }
    }
    for (index, trigger) in session.world.environment.triggers.iter().enumerate() {
        if let Some(p) = drawing.project(point(trigger.position)) {
            drawing.labels.push(Label(
                Anchor::Projected(p),
                format!(
                    "Trigger #{index} type {}{}",
                    trigger.kind,
                    if trigger.clock & 0x8000 != 0 {
                        " active"
                    } else {
                        ""
                    }
                ),
                [1.0, 0.7, 0.3],
            ));
        }
    }
}
