use super::Session;
use crate::debug::{Debug, Overlay, inspection::Actor, overlay::Builder};
use glam::{Mat4, Vec3};
use tenchu_core::tenchu2::collision;

pub(super) fn actors(session: &Session) -> impl Iterator<Item = Actor> + '_ {
    session.world.actors.iter().enumerate().map(|(id, agent)| {
        let actor = &agent.actor;
        Actor {
            id,
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
