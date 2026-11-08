use super::{Request, StatusResult};
use crate::{
    Result,
    math::Position,
    random::Random,
    scene::Scene,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        ai::direction,
        collision::{Collision, DISABLED, WEAPON},
        rig::{Contacts, Rig},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttackPreparation {
    pub status: StatusResult,
    pub retarget: bool,
    pub interrupted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatCue {
    Voice(u8),
    Rumble { kind: u8, strength: u8 },
    Trail { slot: usize, part: usize },
    EndTrail(usize),
    Clash { opponent: usize, position: Position },
}

pub struct AttackContext<'a> {
    pub rig: &'a Rig,
    pub scene: &'a mut Scene,
    pub collision: &'a mut Collision,
    pub random: &'a mut Random,
}

impl Actor {
    pub fn prepare_attack(
        &mut self,
        request: &mut Request,
        resources: &Resources,
        random: &mut Random,
        is_player: bool,
        controls_enabled: bool,
    ) -> Result<AttackPreparation> {
        let clip = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 attack motion")?)
            .ok_or("Missing Tenchu 2 attack motion")?;
        let mut result = AttackPreparation::default();
        if self.motion.loops == 0 && i32::from(self.motion.frame) == 1 - i32::from(clip.blend) {
            if self.flags & 0x100 != 0 && request.action != 0x916 && random.next15() & 1 != 0 {
                result.status = self.update_status(request, random, is_player, controls_enabled);
                if result.status.effect.is_some() {
                    result.interrupted = true;
                    return Ok(result);
                }
            }
            result.retarget = is_player;
            self.attack.index = resources
                .weapons
                .attacks
                .iter()
                .position(|a| a.clip == clip.id);
        }
        Ok(result)
    }

    pub fn steer_attack(
        &mut self,
        request: Request,
        target: Option<Position>,
        resources: &Resources,
    ) -> Result<()> {
        let steering = self
            .attack
            .index
            .and_then(|index| resources.weapons.attacks.get(index))
            .map_or(0, |attack| i16::from(attack.steering));
        if request.action != 0x916
            && self.motion.frame >= 0
            && self.motion.frame < steering
            && let Some(target) = target
        {
            let delta = target.wrapping_sub(self.position);
            let angle = i32::from(direction(
                &resources.math,
                delta.x,
                delta.z,
                self.rotation.y,
            )?);
            if angle > i32::from(self.turn) {
                self.rotation.y = self.rotation.y.wrapping_add(self.turn);
            } else if angle < -i32::from(self.turn) {
                self.rotation.y = self.rotation.y.wrapping_sub(self.turn);
            }
            self.update_velocity(Velocity::Motion, resources)?;
        }
        Ok(())
    }

    pub fn finish_attack(
        &mut self,
        request: &mut Request,
        input: u16,
        resources: &Resources,
        context: &mut AttackContext<'_>,
        mut cue: impl FnMut(&Actor, &mut AttackContext<'_>, CombatCue) -> Result<()>,
    ) -> Result<()> {
        if self.motion.loops != 0 && request.action != 0x911 {
            self.motion.loops = self.motion.loops.wrapping_add(1);
            if self.motion.loops < 0 {
                return Ok(());
            }
            if request.action == 0x916 || self.motion.loops > 0 {
                request.action = self.normal_motion(input, |actor| {
                    context.rig.root_position(actor, context.scene)
                })?;
                request.velocity = Velocity::Motion;
            }
        }
        if request.action == 0x916 {
            return Ok(());
        }
        if self.motion.action != request.action {
            self.end_attack_contacts(resources, context, &mut cue)?;
            let old = self.motion.action;
            if self
                .change_motion(request.action, request.velocity, resources)?
                .is_some()
            {
                self.commit_root(context.rig.root_position(self, context.scene)?);
                let yaw = resources
                    .clips
                    .get(&self.motion.clip.ok_or("No Tenchu 2 attack motion")?)
                    .and_then(|clip| clip.bones.first())
                    .and_then(|keys| keys.first())
                    .ok_or("Missing Tenchu 2 root rotation")?
                    .value[1];
                if matches!(old, 0x907..=0x90c | 0x915) {
                    self.rotation.y = self
                        .rotation
                        .y
                        .wrapping_add(self.motion.pose.rotation[0][1].wrapping_sub(yaw));
                }
                self.motion.pose.rotation[0][1] = yaw;
                self.end_attack_contacts(resources, context, &mut cue)?;
            }
            return Ok(());
        }
        let Some(attack) = self
            .attack
            .index
            .and_then(|index| resources.weapons.attacks.get(index))
        else {
            return Ok(());
        };
        if self.motion.loops != 0 {
            return Ok(());
        }
        let frame = self.motion.frame;
        if frame == i16::from(attack.start) {
            self.set_attack_contacts(context.rig, context.collision, resources, Contacts::Start)?;
            cue(
                self,
                context,
                CombatCue::Rumble {
                    kind: 1,
                    strength: 2,
                },
            )?;
            if request.action != 0x912 {
                let voice = if self.flags & 0x400 == 0 {
                    2
                } else if context.random.next15() & 1 != 0 {
                    13
                } else {
                    14
                };
                cue(self, context, CombatCue::Voice(voice))?;
            }
        } else if frame == i16::from(attack.end) && self.motion.action != 0x911 {
            self.set_attack_contacts(context.rig, context.collision, resources, Contacts::Stop)?;
        }
        if frame > i16::from(attack.start) && frame < i16::from(attack.end) {
            self.clash_weapons(context, &mut cue)?;
        }
        if frame == i16::from(attack.trail_start) {
            let kind = resources
                .registry
                .actor(self.kind)
                .ok_or("Missing Tenchu 2 actor")?
                .weapon;
            let mut slot = 0;
            for (part, weapon) in resources
                .weapons
                .parts
                .iter()
                .enumerate()
                .filter(|(_, part)| part.kind == kind)
            {
                if weapon.radius != 0 && attack.bones & 1u32.wrapping_shl(weapon.bone as u32) != 0 {
                    if slot >= 4 {
                        return Err("Too many Tenchu 2 weapon trails".into());
                    }
                    if self.attack.trails & (1 << slot) != 0 {
                        cue(self, context, CombatCue::EndTrail(slot))?;
                    }
                    cue(self, context, CombatCue::Trail { slot, part })?;
                    self.attack.trails |= 1 << slot;
                    slot += 1;
                }
            }
            if context.random.next15() & 1 != 0 {
                cue(
                    self,
                    context,
                    CombatCue::Voice(if attack.power < 20 { 10 } else { 11 }),
                )?;
            }
        } else if frame == i16::from(attack.trail_end) && request.action != 0x911 {
            self.end_attack_trails(context, &mut cue)?;
        }
        Ok(())
    }

    fn end_attack_contacts(
        &mut self,
        resources: &Resources,
        context: &mut AttackContext<'_>,
        cue: &mut impl FnMut(&Actor, &mut AttackContext<'_>, CombatCue) -> Result<()>,
    ) -> Result<()> {
        if self.weapon_drawn() {
            self.end_attack_trails(context, cue)?;
        }
        self.set_attack_contacts(context.rig, context.collision, resources, Contacts::Finish)
    }

    fn end_attack_trails(
        &mut self,
        context: &mut AttackContext<'_>,
        cue: &mut impl FnMut(&Actor, &mut AttackContext<'_>, CombatCue) -> Result<()>,
    ) -> Result<()> {
        for slot in 0..4 {
            if self.attack.trails & (1 << slot) != 0 {
                cue(self, context, CombatCue::EndTrail(slot))?;
                self.attack.trails &= !(1 << slot);
            }
        }
        Ok(())
    }

    fn clash_weapons(
        &mut self,
        context: &mut AttackContext<'_>,
        cue: &mut impl FnMut(&Actor, &mut AttackContext<'_>, CombatCue) -> Result<()>,
    ) -> Result<()> {
        let rig = context.rig;
        for &joint in &rig.joints {
            let Some(slot) = context
                .collision
                .index(joint)
                .and_then(|id| context.collision.slot(id))
            else {
                continue;
            };
            if slot.flags != WEAPON {
                continue;
            }
            let position = slot.position;
            let mut contacts = [0; 64];
            let mut count = 0;
            for contact in context.collision.contacts(joint) {
                contacts[count] = contact.index;
                count += 1;
            }
            for &other_index in &contacts[..count] {
                let other = context
                    .collision
                    .slot(other_index)
                    .ok_or("Missing Tenchu 2 weapon contact")?;
                if other.flags != WEAPON {
                    continue;
                }
                let opponent = other.owner.ok_or("Tenchu 2 weapon has no owner")?;
                let midpoint = position.wrapping_add(other.position).map(|x| x / 2);
                context
                    .collision
                    .get_mut(joint)
                    .ok_or("Missing Tenchu 2 weapon")?
                    .flags |= DISABLED;
                context
                    .collision
                    .slot_mut(other_index)
                    .ok_or("Missing Tenchu 2 opposing weapon")?
                    .flags |= DISABLED;
                cue(self, context, CombatCue::Voice(31))?;
                cue(
                    self,
                    context,
                    CombatCue::Clash {
                        opponent,
                        position: midpoint,
                    },
                )?;
                self.motion.loops = -5;
            }
        }
        self.refresh_contact_mask(context.rig, context.collision);
        Ok(())
    }
}
