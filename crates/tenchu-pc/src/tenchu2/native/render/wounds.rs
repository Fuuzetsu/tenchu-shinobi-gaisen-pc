use super::*;
use crate::visuals::{
    blood::{Attachment, Blood, BodyPart, View},
    interpolation::Part as PartId,
};
use glam::Vec3;
use std::collections::BTreeSet;
use tenchu_core::tenchu2::actor::State;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Wounds {
    tick: Option<u32>,
    health: BTreeMap<usize, i16>,
    pending: BTreeSet<usize>,
    deaths: BTreeSet<usize>,
    sounds: Vec<Position>,
}

impl Wounds {
    pub fn clear(&mut self) {
        self.pending.clear();
        self.deaths.clear();
        self.sounds.clear();
    }

    pub fn tick(
        &mut self,
        session: &super::super::Session,
        models: &Models,
        environment: &mut environment::Environment,
        settings: &crate::settings::Blood,
    ) -> Result<()> {
        let world = &session.world;
        environment.configure_blood(settings);
        let blood = &mut environment.blood;
        if !settings.gush(false) {
            self.deaths.clear();
        } else if !settings.gush(true) {
            self.deaths.remove(&world.instances[world.player].rig.root);
        }
        if !settings.wounds()
            && !settings.gush(false)
            && !(settings.weapons && settings.amount != 0)
        {
            self.clear();
            self.health.clear();
            if self.tick != Some(world.tick) {
                self.advance(session, environment, &BTreeMap::new());
            }
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
        let mut dead = BTreeSet::new();
        for (owner, (agent, instance)) in world.actors.iter().zip(&world.instances).enumerate() {
            let root = instance.rig.root;
            let actor = &agent.actor;
            if !settings.wounds()
                || !matches!(actor.state, State::Hurt | State::Dead)
                || actor.state == State::Dead && actor.motion.loops < 0
            {
                self.pending.remove(&root);
            }
            if settings.wounds()
                && self
                    .health
                    .get(&root)
                    .is_some_and(|&old| old > actor.health)
            {
                self.pending.insert(root);
            }
            let gush = settings.gush(owner == world.player)
                && actor.health == 0
                && actor.state == State::Dead
                && !matches!(actor.motion.action, 0xb08 | 0xb0a);
            if gush {
                dead.insert(root);
            }
            if !gush || actor.motion.loops < 0 {
                self.deaths.remove(&root);
            }
            if gush && self.health.get(&root).is_some_and(|&old| old > 0) {
                self.deaths.insert(root);
            }
        }
        self.health = health;
        self.pending.retain(|root| self.health.contains_key(root));
        self.deaths.retain(|root| dead.contains(root));
        blood.retain_gushes(|root| dead.contains(&root));
        blood.retain_attached_parts(|key| match key {
            PartId::Actor { root, .. } => self.health.contains_key(&root),
            _ => false,
        });
        let mut contacts = BTreeMap::new();
        for wound in &session.contact.wounds {
            let Some(instance) = world.instances.get(wound.owner) else {
                continue;
            };
            if settings.wounds() && self.pending.remove(&instance.rig.root) {
                contacts.insert(instance.rig.root, (wound.owner, point(wound.position)));
            }
            if self.deaths.remove(&instance.rig.root) {
                blood.start_gush(
                    instance.rig.root,
                    interpolation::part(
                        instance.rig.root,
                        wound.joint,
                        world.actors[wound.owner].actor.definition_kind(),
                    ),
                    Vec3::ZERO,
                );
            }
        }
        let weapons = if settings.weapons
            && settings.amount != 0
            && (!session.contact.melee_hits.is_empty()
                || !session.contact.wounds.is_empty()
                || blood.weapon_parts().next().is_some())
        {
            weapons::Weapons::new(session, models)?
        } else {
            Default::default()
        };
        let needed: BTreeSet<_> = blood
            .wound_parts()
            .chain(blood.gush_parts())
            .chain(blood.weapon_parts())
            .chain(weapons.hits.iter().copied())
            .collect();
        let mut transforms = BTreeMap::new();
        let mut bodies: BTreeMap<usize, Vec<BodyPart>> = BTreeMap::new();
        if !contacts.is_empty() || !needed.is_empty() {
            models.visit_actors(
                world,
                &session.assets,
                &mut world.scene.clone(),
                None,
                |key, meshes, part, matrix, _| {
                    if needed.contains(&key) {
                        transforms.insert(key, matrix);
                    }
                    if weapons.hits.contains(&key) {
                        blood.coat_weapon(key, matrix, u64::from(world.tick), || {
                            let mut vertices = Vec::new();
                            meshes.visit(
                                &part.geometry,
                                Matrix::default(),
                                false,
                                0,
                                None,
                                |triangle| {
                                    vertices.extend(triangle.into_iter().copied());
                                },
                            );
                            vertices
                        });
                    }
                    let PartId::Actor { root, model, .. } = key else {
                        return;
                    };
                    let Some(&(owner, _)) = contacts.get(&root) else {
                        return;
                    };
                    if !world.instances[owner].rig.joints.contains(&model) {
                        return;
                    }
                    transforms.insert(key, matrix);
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
        }
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
            blood.wound_body(parts, position, approach, settings);
        }
        blood.advance_weapons(u64::from(world.tick), &weapons.live, &transforms);
        self.advance(session, environment, &transforms);
        Ok(())
    }

    fn advance(
        &mut self,
        session: &super::super::Session,
        environment: &mut environment::Environment,
        transforms: &BTreeMap<PartId, Matrix>,
    ) {
        let image = session.assets.sprites.effects[2];
        self.sounds.extend(environment.blood.advance_wounds(
            u64::from(session.world.tick),
            transforms,
            &environment.surfaces,
            &[image; 3],
        ));
        if self.sounds.len() > 8 {
            self.sounds.drain(..self.sounds.len() - 8);
        }
    }

    pub fn sounds(
        &mut self,
        session: &super::super::Session,
    ) -> Result<Vec<tenchu_assets::audio::voice::Sourced<tenchu_core::tenchu2::audio::Sound>>> {
        let mut audio = session.world.audio.clone();
        let mut sounds = Vec::new();
        for position in self.sounds.drain(..) {
            if let Some((direction, volume)) = session
                .world
                .listener()
                .parameters(Some(position), &session.assets.resources.math)?
                && let Some(sound) = audio.play(
                    29,
                    direction,
                    volume,
                    session.world.tick,
                    &session.assets.resources.math,
                )
            {
                sounds.push(tenchu_assets::audio::voice::Sourced {
                    sound,
                    bank: Some(session.bank.clone()),
                });
            }
        }
        Ok(sounds)
    }
}

pub(super) fn geometry(
    blood: &Blood,
    session: &super::super::Session,
    models: &Models,
    pose: Option<&PoseFrame>,
    motion: &crate::visuals::motion::Frame,
    view: View,
) -> Result<render_effects::Geometry> {
    let needed: BTreeSet<_> = blood.attached_parts().chain(blood.weapon_parts()).collect();
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
        &view,
        |key| {
            let &(meshes, geometry, matrix) = poses.get(&key)?;
            Some(Attachment {
                matrix,
                deformation: meshes.deformation(motion, key, geometry, matrix),
            })
        },
        &mut vertices,
    );
    blood.append_weapon_blood(
        &mut vertices,
        &poses
            .iter()
            .map(|(&key, &(_, _, matrix))| (key, matrix))
            .collect(),
        view.alpha,
    );
    blood.append_drops(
        view.camera.unwrap_or(session.world.camera.view),
        view.alpha,
        session.assets.sprites.effects[0],
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
pub(super) mod tests;
