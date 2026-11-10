use super::*;
use crate::visuals::{
    blood::{Attachment, Blood, BodyPart, View},
    interpolation::Part as PartId,
};
use glam::Vec3;
use std::collections::BTreeSet;
use tenchu_core::tenchu2::actor::State;

#[derive(Default)]
pub(super) struct Stains {
    tick: Option<u32>,
    health: BTreeMap<usize, i16>,
    pending: BTreeSet<usize>,
}

impl Stains {
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn tick(
        &mut self,
        session: &super::super::Session,
        models: &Models,
        blood: &mut Blood,
        settings: &crate::settings::Blood,
    ) -> Result<()> {
        let world = &session.world;
        if !settings.model_stains {
            blood.clear_model_stains();
            self.clear();
            self.health.clear();
            self.tick = Some(world.tick);
            return Ok(());
        }
        if self.tick == Some(world.tick) && !self.health.is_empty() {
            return Ok(());
        }
        let health: BTreeMap<_, _> = world
            .actors
            .iter()
            .zip(&world.instances)
            .map(|(a, i)| (i.rig.root, a.actor.health))
            .collect();
        if self.health.is_empty()
            || self
                .tick
                .is_none_or(|old| old.wrapping_add(1) != world.tick)
        {
            self.clear();
            self.health = health;
            self.tick = Some(world.tick);
            return Ok(());
        }
        self.tick = Some(world.tick);
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            let root = instance.rig.root;
            let actor = &agent.actor;
            if !matches!(actor.state, State::Hurt | State::Dead)
                || actor.state == State::Dead && actor.motion.loops < 0
            {
                self.pending.remove(&root);
            }
            if self
                .health
                .get(&root)
                .is_some_and(|&old| old > actor.health)
            {
                self.pending.insert(root);
            }
        }
        self.health = health;
        self.pending.retain(|root| self.health.contains_key(root));
        blood.retain_attached_parts(|key| match key {
            PartId::Actor { root, .. } => self.health.contains_key(&root),
            _ => false,
        });
        let mut contacts = BTreeMap::new();
        for &(owner, position) in &session.contact.wounds {
            if let Some(instance) = world.instances.get(owner)
                && self.pending.remove(&instance.rig.root)
            {
                contacts.insert(instance.rig.root, (owner, point(position)));
            }
        }
        if contacts.is_empty() {
            return Ok(());
        }
        let mut bodies: BTreeMap<usize, Vec<BodyPart>> = BTreeMap::new();
        models.visit_actors(
            world,
            &session.assets,
            &mut world.scene.clone(),
            None,
            |key, meshes, part, matrix, _| {
                let PartId::Actor { root, model, .. } = key else {
                    return;
                };
                let Some(&(owner, _)) = contacts.get(&root) else {
                    return;
                };
                if !world.instances[owner].rig.joints.contains(&model) {
                    return;
                }
                let mut vertices = Vec::new();
                meshes.visit(&part.geometry, matrix, false, 0, None, |triangle| {
                    vertices.extend(triangle.into_iter().copied());
                });
                bodies.entry(root).or_default().push(BodyPart {
                    key,
                    matrix,
                    vertices,
                });
            },
        )?;
        for (root, (owner, position)) in contacts {
            let Some(parts) = bodies.get(&root) else {
                continue;
            };
            let approach = world
                .actors
                .iter()
                .enumerate()
                .filter(|(id, a)| *id != owner && a.actor.health > 0 && a.actor.flags & 8 == 0)
                .map(|(_, a)| {
                    Vec3::new(
                        a.actor.position.x as f32,
                        position.y,
                        a.actor.position.z as f32,
                    )
                })
                .filter(|p| p.distance_squared(position) < 3000.0_f32.powi(2))
                .min_by(|a, b| {
                    a.distance_squared(position)
                        .total_cmp(&b.distance_squared(position))
                })
                .map(|p| (p - position).normalize_or_zero());
            blood.stain_body(parts, position, approach, settings);
        }
        Ok(())
    }
}

pub(super) fn geometry(
    blood: &Blood,
    session: &super::super::Session,
    models: &Models,
    pose: Option<&PoseFrame>,
    motion: &crate::visuals::motion::Frame,
    projection: glam::Mat4,
) -> Result<render_effects::Geometry> {
    let needed: BTreeSet<_> = blood.attached_parts().collect();
    let mut poses = BTreeMap::new();
    if !needed.is_empty() {
        models.visit_actors(
            &session.world,
            &session.assets,
            &mut session.world.scene.clone(),
            pose,
            |key, meshes, part, matrix, style| {
                if needed.contains(&key)
                    && !matches!(
                        style,
                        ModelStyle::Concealed | ModelStyle::Imported { concealed: true }
                    )
                {
                    poses.insert(key, (meshes, part.geometry.as_ptr() as usize, matrix));
                }
            },
        )?;
    }
    let mut vertices = Vec::new();
    blood.append_attached(
        &View {
            projection: Some(projection),
            ..Default::default()
        },
        |key| {
            let &(meshes, geometry, matrix) = poses.get(&key)?;
            Some(Attachment {
                matrix,
                deformation: meshes.deformation(motion, key, geometry, matrix),
            })
        },
        &mut vertices,
    );
    let count = vertices.len() as u32;
    Ok(render_effects::Geometry {
        vertices,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..count,
        }],
    })
}

fn point(position: Position) -> Vec3 {
    Vec3::from(position.to_array().map(|v| v as f32))
}

#[cfg(test)]
mod tests;
