use super::{
    actor::{Actor, Resources},
    terrain::{self, Cursor, Probe, Query},
    world::{Assets, World},
};
use crate::{
    Result,
    game::dismemberment::{Part, Settings},
    math::{Matrix, Position},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{BONES, registry::StealthEffect};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Cause {
    Combat { dive: bool },
    RearStealth,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Death {
    pub pending: Option<Cause>,
    pub part: Option<Part>,
}

impl Actor {
    fn can_sever(&self, settings: Settings, player: bool) -> bool {
        if player {
            settings.player
        } else {
            settings.enabled && self.kind != 0x1a && matches!(self.kind & 0xf0, 0x10..=0x60)
        }
    }

    pub(crate) fn note_combat_death(&mut self, settings: Settings, player: bool, dive: bool) {
        if self.can_sever(settings, player) {
            self.dismemberment = Some(Death {
                pending: Some(Cause::Combat { dive }),
                part: None,
            });
        }
    }

    pub fn severed(&self) -> Option<&Part> {
        self.dismemberment.as_ref()?.part.as_ref()
    }

    pub(crate) fn attached_mask(&self) -> u32 {
        self.severed().map_or(u32::MAX, |part| {
            !((1 << part.index) | (1 << BONES[13]) | (1 << BONES[14]))
        })
    }
}

impl World {
    pub(crate) fn severing_frame(&mut self, index: usize, resources: &Resources) -> bool {
        let actor = &mut self.actors[index].actor;
        if actor.health != 0 {
            return false;
        }
        if actor.dismemberment.is_none()
            && actor.motion.action == 0xb11
            && actor.can_sever(self.controls.dismemberment, index == self.player)
        {
            actor.dismemberment = Some(Death {
                pending: Some(Cause::RearStealth),
                part: None,
            });
        }
        match actor.dismemberment.as_ref().and_then(|d| d.pending) {
            Some(Cause::Combat { .. }) => {
                matches!(actor.motion.action, 0xa21..=0xa25 | 0xb00..=0xb05)
                    && actor.motion.frame == 5
            }
            Some(Cause::RearStealth) if actor.motion.action == 0xb11 => self
                .controls
                .stealth
                .as_ref()
                .and_then(|s| {
                    resources
                        .registry
                        .stealth_events(s.playback.hero, s.playback.style)?
                        .get(s.playback.next)
                })
                .is_some_and(|e| {
                    e.frame == actor.motion.frame && matches!(e.effect, StealthEffect::Blood { .. })
                }),
            _ => false,
        }
    }

    pub(crate) fn update_dismemberment(
        &mut self,
        index: usize,
        assets: &Assets,
        severing: bool,
    ) -> Result<()> {
        self.advance_severed(index, assets)?;
        let actor = &mut self.actors[index].actor;
        let settings = self.controls.dismemberment;
        if !severing || actor.health != 0 {
            return Ok(());
        }
        let Some(cause) = actor.dismemberment.as_mut().and_then(|d| d.pending.take()) else {
            return Ok(());
        };
        if !actor.can_sever(settings, index == self.player) || actor.motion.pose.rotation.len() < 15
        {
            return Ok(());
        }
        let part = if matches!(cause, Cause::RearStealth) {
            Settings::head(settings.stealth_head, &mut self.random).then_some(2)
        } else {
            let (head, arms, side) = if index == self.player {
                (
                    Some(settings.player_head),
                    settings.player_arms,
                    settings.player_right_arm,
                )
            } else {
                (
                    matches!(cause, Cause::Combat { dive: true }).then_some(settings.dive_head),
                    settings.arms,
                    settings.right_arm,
                )
            };
            if head.is_some_and(|chance| Settings::head(chance, &mut self.random)) {
                Some(2)
            } else if Settings::arm(arms, &mut self.random) {
                Some(if Settings::arm(side, &mut self.random) {
                    BONES[4]
                } else {
                    BONES[6]
                })
            } else {
                None
            }
        };
        let Some(part) = part else { return Ok(()) };
        let rig = &self.instances[index].rig;
        let joint = rig.joints[part];
        let position = self.scene.world(joint)?.translation;
        let parent = self.scene.models[joint].parent;
        let rotation = [0, actor.rotation.y, 0];
        let mut velocity = if part == 2 {
            assets.resources.math.velocity(actor.rotation.y, 100, 0)
        } else {
            [
                (self.random.next15() % 100 - 50) as i16,
                0,
                (self.random.next15() % 100 - 50) as i16,
            ]
        };
        velocity[1] = -100;
        self.scene.models[joint].parent = Some(0);
        self.scene.models[joint].attribute |= 2;
        self.scene.update(
            joint,
            Matrix {
                translation: position,
                rotation: assets.resources.math.rotate_yxz(rotation),
            },
        );
        actor.dismemberment.as_mut().unwrap().part = Some(Part {
            index: part,
            parent,
            velocity,
            rotation,
            moving: true,
        });
        actor.motion.mask &= actor.attached_mask();
        Ok(())
    }

    fn advance_severed(&mut self, index: usize, assets: &Assets) -> Result<()> {
        if self.actors[index].actor.health != 0 {
            if self.restore_dismemberment(index) {
                self.publish(index, assets);
            }
            return Ok(());
        }
        let actor = &mut self.actors[index].actor;
        let kind = actor.definition_kind();
        let Some(part) = actor.dismemberment.as_mut().and_then(|d| d.part.as_mut()) else {
            return Ok(());
        };
        let rig = &self.instances[index].rig;
        let joint = rig.joints[part.index];
        if !part.moving {
            return Ok(());
        }
        let definition = assets
            .resources
            .registry
            .actor(kind)
            .ok_or("Missing severed actor definition")?;
        let body = &assets.models[&definition.model].parts[part.index];
        let mut matrix = self.scene.models[joint].local;
        let mut position = matrix.translation;
        let mut cursor = Cursor::default();
        let floor = supported_height(body, matrix, &self.terrain, &mut cursor)?;
        if floor.is_some_and(|floor| position.y >= floor) {
            let mut probe = Probe::default();
            probe.sample(
                &self.terrain,
                &mut cursor,
                &assets.terrain_rules,
                position,
                actor.width,
                Default::default(),
            )?;
            let [x, z] = assets.terrain_rules.escape[usize::from(probe.blocked)];
            if x != 0 || z != 0 {
                part.velocity[0] = x.wrapping_mul(-50);
                part.velocity[2] = z.wrapping_mul(-50);
            } else {
                part.velocity[0] /= 2;
                part.velocity[2] /= 2;
            }
            if part.index != 2 {
                part.rotation[0] = 1024;
                matrix.rotation = assets.resources.math.rotate_yxz(part.rotation);
            }
            position.y =
                supported_height(body, matrix, &self.terrain, &mut cursor)?.unwrap_or(position.y);
            part.velocity[1] = part.velocity[1].wrapping_neg() / 2;
            if part.velocity[1].unsigned_abs() < 20 {
                part.moving = false;
                self.scene.models[joint].attribute &= !2;
            }
        } else {
            part.velocity[1] = part.velocity[1].wrapping_add(20);
        }
        position = position.wrapping_add(part.velocity.map(i32::from).into());
        if part.index == 2 && part.moving {
            part.rotation[0] = part.rotation[0].wrapping_add(part.velocity[1].wrapping_abs());
        }
        if !part.moving && part.index == 2 {
            part.rotation = self
                .controls
                .dismemberment
                .head_rest
                .rotation(part.rotation, rig.root);
        }
        matrix.translation = position;
        matrix.rotation = assets.resources.math.rotate_yxz(part.rotation);
        if let Some(floor) = supported_height(body, matrix, &self.terrain, &mut cursor)? {
            matrix.translation.y = if part.moving {
                matrix.translation.y.min(floor)
            } else {
                floor
            };
        }
        self.scene.update(joint, matrix);
        Ok(())
    }

    pub(crate) fn restore_dismemberment(&mut self, index: usize) -> bool {
        let actor = &mut self.actors[index].actor;
        let Some(part) = actor.dismemberment.take().and_then(|d| d.part) else {
            return false;
        };
        let joint = self.instances[index].rig.joints[part.index];
        self.scene.models[joint].parent = part.parent;
        self.scene.models[joint].attribute &= !2;
        actor.motion.mask |= (1 << part.index) | (1 << BONES[13]) | (1 << BONES[14]);
        true
    }
}

fn supported_height(
    part: &tenchu_assets::model::Part,
    matrix: Matrix,
    map: &tenchu_assets::tenchu2::collision::Collision,
    cursor: &mut Cursor,
) -> Result<Option<i32>> {
    let mut height = None;
    for vertex in part.geometry.iter().flat_map(|mesh| &mesh.vertices) {
        let point = matrix.transform(*vertex);
        // SA's STEP_DOWN query subtracts 150 after conversion to terrain units
        // and rejects a surface more than 1000 world units above the vertex.
        let query = Position::new(
            point.x,
            (point.y / 10).wrapping_sub(150).wrapping_mul(10),
            point.z,
        );
        if let Some(floor) = terrain::level(map, cursor, query, Query::Uncached)?.filter(|floor| {
            floor.wrapping_sub(point.y) >= -1000 && cursor.material & 0xf00 != 0x100
        }) {
            let limit = floor
                .wrapping_sub(point.y.wrapping_sub(matrix.translation.y))
                .wrapping_sub(2);
            height = Some(height.map_or(limit, |old: i32| old.min(limit)));
        }
    }
    Ok(height)
}
