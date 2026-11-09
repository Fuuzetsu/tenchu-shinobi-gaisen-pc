use super::{
    actor::{Actor, Resources},
    collision::{BODY, Collision, DISABLED, WEAPON},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    scene::{ModelId, Scene},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rig {
    pub root: ModelId,
    pub joints: Vec<ModelId>,
    pub weapons: [Option<ModelId>; 5],
}

impl Rig {
    pub fn root_position(&self, actor: &Actor, scene: &mut Scene) -> Result<Position> {
        let joint = *self.joints.first().ok_or("Missing Tenchu 2 root joint")?;
        scene.set_translation(self.root, actor.position);
        scene.set_translation(joint, actor.motion.pose.translation[0]);
        // Source translations alias the actor and pose; writes don't invalidate cached matrices.
        Ok(scene.world(joint)?.translation)
    }

    pub fn bind(actor: &Actor, scene: &mut Scene, math: &Math) -> Self {
        let root = scene.add(
            Some(0),
            Matrix {
                rotation: math.rotate_yxz(actor.rotation.to_array()),
                translation: actor.position,
            },
        );
        Self {
            root,
            joints: actor.motion.pose.bind(scene, root, math),
            weapons: [None; 5],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponMode {
    Draw,
    Sheathe,
    Refresh,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip {
    Swap,
    Hold,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contacts {
    Start,
    Stop,
    Finish,
}

impl Actor {
    pub fn swap_attachments(
        &mut self,
        rig: &mut Rig,
        a: usize,
        b: usize,
        grip: Grip,
    ) -> Result<Option<u8>> {
        if a >= rig.weapons.len() || b >= rig.weapons.len() {
            return Err("Invalid Tenchu 2 weapon slot".into());
        }
        let sound = match grip {
            Grip::Release if self.flags & 0x200000 == 0 => return Ok(None),
            Grip::Release => {
                self.flags &= !0x200000;
                Some(1)
            }
            Grip::Hold if self.flags & 0x200000 != 0 => return Ok(None),
            Grip::Hold => {
                self.flags |= 0x200000;
                Some(0)
            }
            Grip::Swap => None,
        };
        rig.weapons.swap(a, b);
        Ok(sound)
    }

    pub fn set_weapon(
        &mut self,
        rig: &mut Rig,
        owner: usize,
        collision: &mut Collision,
        resources: &Resources,
        mode: WeaponMode,
    ) -> Result<()> {
        if (mode == WeaponMode::Draw && self.weapon_drawn())
            || (mode == WeaponMode::Sheathe && !self.weapon_drawn())
        {
            return Ok(());
        }
        let kind = resources
            .registry
            .actor(self.definition_kind())
            .ok_or("Missing Tenchu 2 actor definition")?
            .weapon;
        if matches!(mode, WeaponMode::Draw | WeaponMode::Sheathe) {
            self.flags ^= 0x40;
            if kind & 0xf00 == 0x200 {
                self.swap_attachments(rig, 1, 4, Grip::Swap)?;
            }
            if matches!(kind & 0xf00, 0x100 | 0x200) {
                self.swap_attachments(rig, 0, 3, Grip::Swap)?;
            }
        }
        if matches!(mode, WeaponMode::Sheathe | WeaponMode::Remove) {
            for &joint in &rig.joints {
                if collision
                    .get_mut(joint)
                    .is_some_and(|slot| slot.flags & WEAPON != 0)
                {
                    collision.remove(joint);
                }
            }
        } else {
            for part in resources
                .weapons
                .parts(kind)
                .filter(|part| part.radius != 0)
            {
                let joint = *rig
                    .joints
                    .get(part.bone)
                    .ok_or("Missing Tenchu 2 weapon joint")?;
                let slot = collision.insert(joint)?;
                slot.owner = Some(owner);
                slot.offset = part.center.into();
                slot.size = Vector3::new(part.radius, part.radius, part.radius);
                slot.flags = WEAPON | DISABLED;
            }
        }
        self.refresh_contact_mask(rig, collision);
        Ok(())
    }

    pub fn set_attack_contacts(
        &mut self,
        rig: &Rig,
        collision: &mut Collision,
        resources: &Resources,
        mode: Contacts,
    ) -> Result<()> {
        if !self.weapon_drawn() {
            return Ok(());
        }
        if mode == Contacts::Start
            && let Some(attack) = self.imported_attack()
        {
            let owner = rig
                .joints
                .first()
                .and_then(|&joint| collision.get_mut(joint))
                .and_then(|slot| slot.owner);
            for contact in attack.contacts.iter().flatten() {
                let joint = *rig
                    .joints
                    .get(contact.part)
                    .ok_or("Missing imported weapon bone")?;
                let slot = collision.insert(joint)?;
                slot.owner = owner;
                slot.offset = contact.hitbox.center.into();
                slot.size = Vector3::new(
                    contact.hitbox.radius,
                    contact.hitbox.radius,
                    contact.hitbox.radius,
                );
                slot.flags = WEAPON | DISABLED;
            }
        }
        let mask = if mode == Contacts::Start {
            resources
                .weapons
                .attacks
                .get(
                    self.attack
                        .index
                        .ok_or("Missing Tenchu 2 attack selection")?,
                )
                .ok_or("Missing Tenchu 2 attack")?
                .bones
        } else {
            u32::MAX
        };
        if mode == Contacts::Finish {
            self.attack.trails = 0;
        }
        for (bone, &joint) in rig.joints.iter().enumerate() {
            if mask & 1u32.wrapping_shl(bone as u32) != 0
                && let Some(slot) = collision.get_mut(joint)
                && slot.flags & WEAPON != 0
            {
                slot.flags = WEAPON | if mode == Contacts::Start { 0 } else { DISABLED };
            }
        }
        self.refresh_contact_mask(rig, collision);
        Ok(())
    }

    pub(crate) fn refresh_contact_mask(&mut self, rig: &Rig, collision: &Collision) {
        self.attack.contacts = rig
            .joints
            .iter()
            .enumerate()
            .fold(0, |mask, (bone, &joint)| {
                if collision
                    .index(joint)
                    .and_then(|i| collision.slot(i))
                    .is_some_and(|slot| slot.flags & (WEAPON | DISABLED) == WEAPON)
                {
                    mask | 1u32.wrapping_shl(bone as u32)
                } else {
                    mask
                }
            });
    }

    pub fn set_body_collision(
        &self,
        joint: ModelId,
        owner: usize,
        collision: &mut Collision,
        enabled: bool,
    ) -> Result<()> {
        if enabled {
            let slot = collision.insert(joint)?;
            slot.size = Vector3::new(self.width / 2, self.height / 2, self.width / 2);
            if self.health == 0 {
                slot.size.y /= 4;
            }
            slot.owner = Some(owner);
            slot.flags = BODY;
        } else {
            collision.remove(joint);
        }
        Ok(())
    }
}
