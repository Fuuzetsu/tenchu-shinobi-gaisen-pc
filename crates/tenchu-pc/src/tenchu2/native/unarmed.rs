use super::*;
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    executable::Executable,
    game::GameAssets,
    tenchu2::{BONES, motions},
    weapon::{Contact, Hitbox, ImportedAttack},
};
use tenchu_core::{
    game::{
        moveset::T2_ACTIONS,
        sheathing::{Settings, UnarmedStyle},
    },
    tenchu2::sheathing::{Unarmed, UnarmedMoves},
};

impl Session {
    pub fn needs_unarmed(&self, settings: Settings) -> bool {
        let actor = &self.world.actors[self.world.player].actor;
        settings.enabled
            && settings.unarmed
            && !actor.unarmed_attack()
            && actor
                .unarmed
                .as_ref()
                .is_none_or(|u| u.moves.style != settings.unarmed_style)
    }

    pub fn configure_tatsumaru(
        &mut self,
        disc: &Disc<impl ReadAt>,
        settings: Settings,
    ) -> Result<()> {
        if !self.needs_unarmed(settings) || settings.unarmed_style != UnarmedStyle::Tatsumaru {
            return Ok(());
        }
        let resources = &mut self.assets.resources;
        let definition = resources
            .registry
            .actor(2)
            .ok_or("Missing Tatsumaru definition")?;
        let mut set = UnarmedMoves {
            style: UnarmedStyle::Tatsumaru,
            rest_yaw: None,
            motions: BTreeMap::new(),
            attacks: BTreeMap::new(),
        };
        for entry in definition.motions.iter().filter(|m| grounded(m.action)) {
            let Some(attack) = resources.weapons.attack(entry.clip).filter(|a| a.start > 0) else {
                continue;
            };
            let mut contacts = resources
                .weapons
                .parts(definition.weapon)
                .filter(|p| p.radius > 0 && attack.bones & (1 << p.bone) != 0)
                .map(|p| Contact {
                    part: p.bone,
                    hitbox: Hitbox {
                        kind: p.kind,
                        center: p.center,
                        radius: p.radius,
                        trail: p.trail,
                    },
                });
            let metadata = ImportedAttack {
                contacts: [contacts.next(), contacts.next()],
                combo: [None; 3],
                turn_on_exit: false,
                holster: None,
            };
            if metadata.contacts[0].is_none() || contacts.next().is_some() {
                return Err("Unsupported Tatsumaru striking limbs".into());
            }
            set.motions.insert(entry.action, entry.clip);
            set.attacks.insert(entry.action, metadata);
        }
        if !set.motions.contains_key(&0x900) {
            return Err("Missing Tatsumaru punch".into());
        }
        let idle = resources
            .registry
            .clip(2, 0x100)
            .ok_or("Missing Tatsumaru combat stance")?;
        let mut wanted: std::collections::BTreeSet<_> = set.motions.values().copied().collect();
        wanted.insert(idle);
        let missing = wanted
            .iter()
            .filter(|id| !resources.clips.contains_key(id))
            .copied()
            .collect();
        for (id, clip) in motions::collect(disc, missing)? {
            resources
                .clips
                .insert(id, clip.at_gameplay_rate(resources.registry.edition));
        }
        set.rest_yaw = Some(resources.clips[&idle].bones[0][0].value[1]);
        self.world.actors[self.world.player].actor.unarmed = Some(Unarmed {
            moves: Arc::new(set),
            active: false,
        });
        Ok(())
    }

    pub fn configure_onikage(
        &mut self,
        assets: &GameAssets<impl ReadAt>,
        settings: Settings,
    ) -> Result<()> {
        if !self.needs_unarmed(settings) || settings.unarmed_style != UnarmedStyle::Onikage {
            return Ok(());
        }
        let source = crate::sheathing::onikage_attacks(assets)?;
        let weapons =
            tenchu_assets::weapon::Weapons::load(assets, &Executable::new(&assets.executable)?)?;
        let hitbox = weapons
            .hitboxes
            .iter()
            .find(|h| h.kind == tenchu_assets::weapon::kind::FIST)
            .ok_or("Missing Gaisen unarmed contacts")?;
        let mut set = UnarmedMoves {
            style: UnarmedStyle::Onikage,
            rest_yaw: None,
            motions: BTreeMap::new(),
            attacks: BTreeMap::new(),
        };
        let mut clips = BTreeMap::new();
        let mut attacks = BTreeMap::new();
        for (action, clip) in source {
            let Some(&(_, action)) = T2_ACTIONS
                .iter()
                .find(|&&(g, n)| g == action && grounded(n))
            else {
                continue;
            };
            let attack = weapons
                .attacks
                .iter()
                .find(|a| a.clip == clip.id && a.start > 0)
                .ok_or("Missing Onikage hit timing")?;
            let id = clip
                .id
                .checked_add(0x6000)
                .filter(|_| (0..0x1000).contains(&clip.id))
                .ok_or("Invalid Onikage clip ID")?;
            clips.insert(id, super::models::gaisen_clip(&clip, id));
            let metadata = ImportedAttack {
                contacts: [
                    Some(Contact {
                        part: BONES[8],
                        hitbox: hitbox.clone(),
                    }),
                    Some(Contact {
                        part: BONES[11],
                        hitbox: hitbox.clone(),
                    }),
                ],
                combo: match action {
                    0x900 => [Some(0x901), Some(0x902), Some(0x903)],
                    0x901 => [Some(0x904); 3],
                    0x904 => [Some(0x905); 3],
                    0x907 => [None, Some(0x908), Some(0x909)],
                    0x90a => [None, Some(0x90c), Some(0x90b)],
                    _ => [None; 3],
                },
                turn_on_exit: true,
                holster: None,
            };
            let mut timing =
                super::models::gaisen_attack(attack, id, (1 << BONES[8]) | (1 << BONES[11]));
            timing.trail_start = 0;
            timing.trail_end = 0;
            attacks.insert(id, timing);
            set.motions.insert(action, id);
            set.attacks.insert(action, metadata);
        }
        for (alias, action) in [(0x902, 0x901), (0x903, 0x901), (0x909, 0x908)] {
            if let Some(&clip) = set.motions.get(&action) {
                set.motions.insert(alias, clip);
                let mut attack = set.attacks[&action].clone();
                attack.combo = [None; 3];
                set.attacks.insert(alias, attack);
            }
        }
        for attack in set.attacks.values_mut() {
            for next in &mut attack.combo {
                *next = next.filter(|a| set.motions.contains_key(a));
            }
        }
        if !set.motions.contains_key(&0x900) {
            return Err("Missing Onikage attack".into());
        }
        self.assets.resources.clips.extend(clips);
        for attack in attacks.into_values() {
            if let Some(existing) = self
                .assets
                .resources
                .weapons
                .attacks
                .iter_mut()
                .find(|a| a.clip == attack.clip)
            {
                *existing = attack;
            } else {
                self.assets.resources.weapons.attacks.push(attack);
            }
        }
        self.world.actors[self.world.player].actor.unarmed = Some(Unarmed {
            moves: Arc::new(set),
            active: false,
        });
        Ok(())
    }
}

fn grounded(action: i16) -> bool {
    matches!(action, 0x900..=0x910 | 0x913..=0x915)
}

impl<S: ReadAt> campaign::Playback<S> {
    pub fn configure_onikage(
        &mut self,
        assets: &GameAssets<impl ReadAt>,
        settings: &crate::settings::Settings,
    ) -> Result<()> {
        if let campaign::Content::Gameplay(session) = &mut self.content {
            session.configure_onikage(assets, settings.sheathing)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
