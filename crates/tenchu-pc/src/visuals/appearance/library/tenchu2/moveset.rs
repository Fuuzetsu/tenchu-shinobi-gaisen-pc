use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    executable::{Character, MotionRegistration},
    tenchu2::{self, Edition, motions, registry::Registry, weapons::Weapons},
    weapon::{
        Attachment, Attack, Contact, Draw, Equipment, Hitbox, ImportedAttack, ImportedModel, kind,
    },
};
use tenchu_core::{game::moveset::Imported, ids::*};

use tenchu_core::game::moveset::T2_ACTIONS as ACTIONS;

pub fn hero(name: &str) -> Option<i16> {
    match name {
        "rikimaru" | "rikia" => Some(0),
        "ayame" | "ayamea" => Some(1),
        "tatumaru" | "tatua" | "tatua_m" => Some(2),
        _ => None,
    }
}

pub fn load(
    disc: &Disc<impl ReadAt>,
    registry: &Registry,
    voices: &BTreeMap<i16, Arc<tenchu_assets::audio::voice::VoiceSet>>,
) -> Result<BTreeMap<i16, Arc<Imported>>> {
    // Both discs store authored 30 Hz tracks. PAL's 25 Hz load-time patch is
    // inappropriate for Gaisen's 30 Hz controller (as in the Tatsumaru importer).
    let weapons = Weapons::load(disc)?;
    let clips = motions::collect(
        disc,
        registry
            .actors
            .iter()
            .map(|actor| actor.kind)
            .flat_map(|kind| {
                ACTIONS
                    .iter()
                    .filter_map(move |&(_, action)| registry.clip(kind, action))
            })
            .collect(),
    )?;
    let mut out = BTreeMap::new();
    // 0x6000..0x6800 belongs to the separate Tatsumaru unarmed importer.
    let base_npc: i16 = match registry.edition {
        Edition::Japan => 0x6800,
        Edition::Europe => 0x7400,
    };
    let mut next_id = base_npc;
    let vram = Arc::new(tenchu2::weapons::textures(disc)?);
    let mut models = BTreeMap::new();
    for actor in &registry.actors {
        let kind = actor.kind;
        let base: i16 = match registry.edition {
            Edition::Japan => 0x3000,
            Edition::Europe => 0x4800,
        } + kind.min(2) * 0x800;
        let mut character = Character {
            kind: if kind < 3 {
                base + kind
            } else {
                base_npc + kind
            },
            weapon: if kind < 3 {
                [kind::NINJA, kind::KODATI, kind::FIST][kind as usize]
            } else {
                proxy_weapon(actor.weapon, &weapons)
            },
            turn: actor.turn,
            health: actor.health,
            width: 0,
            height: 0,
            model: actor.model.clone(),
            motions: Vec::new(),
        };
        let mut converted = BTreeMap::new();
        let mut attacks = Vec::new();
        let mut weapon_frames = Vec::new();
        let mut ids = BTreeMap::new();
        for &(action, original) in ACTIONS {
            let Some(id) = registry.clip(kind, original) else {
                continue;
            };
            if !(0..0x800).contains(&id) {
                return Err("Tenchu 2 motion exceeds reserved import range".into());
            }
            let Some(source) = clips.get(&id) else {
                if kind < 3 {
                    return Err(format!("Missing Tenchu 2 motion {id:#x}"));
                }
                continue;
            };
            let mut clip = source.gaisen_pose();
            clip.id = if kind < 3 {
                base + id
            } else if let Some(&mapped) = ids.get(&id) {
                mapped
            } else {
                let mapped = next_id;
                next_id = next_id
                    .checked_add(1)
                    .ok_or("Tenchu 2 motion ID out of range")?;
                if registry.edition == Edition::Japan && next_id >= 0x7400 {
                    return Err("Tenchu 2 motion import range exhausted".into());
                }
                ids.insert(id, mapped);
                mapped
            };
            clip.sweep = (-i32::from(source.blend))
                .try_into()
                .map_err(|_| "Tenchu 2 blend out of range")?;
            clip.forward_speed = source.forward_speed;
            clip.side_speed = source.side_speed;
            if matches!(original, 0x700 | 0x701) {
                weapon_frames.push((
                    clip.id,
                    tenchu_assets::weapon::WeaponTransition {
                        frame: registry.weapon_frame_30hz(id, source.duration),
                        sound: weapons
                            .parts(actor.weapon)
                            .next()
                            .is_some_and(|part| part.model.is_some()),
                    },
                ));
            }
            if original >= 0x900 && original != 0x916 {
                let Some(source_attack) = weapons.attack(id) else {
                    continue;
                };
                // Projectile and scripted boss attacks need their own event adapter;
                // never advertise them as working melee just because they animate.
                if kind >= 3 && source_attack.start == 0 {
                    continue;
                }
                if kind >= 3
                    && weapons.parts(actor.weapon).any(|p| {
                        p.radius > 0
                            && source_attack.bones & (1 << p.bone) != 0
                            && !tenchu2::BONES.contains(&p.bone)
                    })
                {
                    continue;
                }
                let contacts = weapons
                    .parts(actor.weapon)
                    .filter(|p| p.radius > 0 && source_attack.bones & (1 << p.bone) != 0)
                    .map(|p| {
                        Ok(Contact {
                            part: tenchu2::BONES
                                .iter()
                                .position(|&bone| bone == p.bone)
                                .ok_or("Unsupported Tenchu 2 contact bone")?,
                            hitbox: Hitbox {
                                kind: character.weapon,
                                center: p.center,
                                radius: p.radius,
                                trail: p.trail,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                if contacts.len() > 2 || contacts.is_empty() && source_attack.start > 0 {
                    if kind >= 3 {
                        continue;
                    }
                    return Err(format!(
                        "Unsupported Tenchu 2 attack contacts {id:#x}: {}",
                        contacts.len()
                    ));
                }
                let mut contacts = contacts.into_iter();
                let combo = combo(original)
                    .map(|next| next.filter(|&next| character_action_exists(registry, kind, next)));
                let trail = source_attack.trail_start > 0 && kind != 2;
                attacks.push((
                    Attack {
                        clip: clip.id,
                        power: i16::from(source_attack.power),
                        start: i16::from(source_attack.start),
                        end: i16::from(source_attack.end),
                        combo: i16::from(source_attack.combo),
                        steering: i16::from(source_attack.steering),
                        trail_start: if trail {
                            i16::from(source_attack.trail_start)
                        } else {
                            0
                        },
                        trail_end: if trail {
                            i16::from(source_attack.trail_end)
                        } else {
                            0
                        },
                    },
                    ImportedAttack {
                        contacts: [contacts.next(), contacts.next()],
                        combo,
                        turn_on_exit: matches!(original, 0x907..=0x90c | 0x915),
                        holster: holster(id),
                    },
                ));
            }
            character.motions.push(MotionRegistration {
                action,
                clip: clip.id,
            });
            converted.insert(clip.id, clip);
        }
        if kind >= 3 && !character.motions.iter().any(|m| m.action == MOT_ATTACK) {
            continue;
        }
        let supported: std::collections::BTreeSet<_> =
            character.motions.iter().map(|m| m.action).collect();
        for (_, attack) in &mut attacks {
            for next in &mut attack.combo {
                *next = next.filter(|action| supported.contains(action));
            }
        }
        out.insert(
            kind,
            Arc::new(Imported {
                voice: voices.get(&kind).cloned(),
                character,
                clips: converted.into_values().collect(),
                weapon_frames,
                attacks,
                equipment: equipment(disc, &weapons, actor.weapon, &vram, &mut models)?,
            }),
        );
    }
    Ok(out)
}

fn equipment(
    disc: &Disc<impl ReadAt>,
    weapons: &Weapons,
    kind: i16,
    vram: &Arc<tenchu_assets::texture::Vram>,
    models: &mut BTreeMap<String, Arc<ImportedModel>>,
) -> Result<Equipment> {
    let mut attachments = Vec::new();
    for part in weapons.parts(kind) {
        let Some(name) = &part.model else { continue };
        let model = if let Some(model) = models.get(name) {
            model.clone()
        } else {
            let mut parts = tenchu2::weapons::model(disc, name)?;
            if parts.len() != 1 || parts[0].parent != -1 || parts[0].position != [0; 3] {
                return Err(format!("Unsupported articulated imported weapon: {name}"));
            }
            let model = Arc::new(ImportedModel {
                id: format!("{}weapons/{name}", weapons.edition.model_prefix()),
                source_id: format!("tenchu2/weapons/{name}/0"),
                geometry: parts.remove(0).geometry,
                vram: vram.clone(),
            });
            models.insert(name.clone(), model.clone());
            model
        };
        attachments.push(Attachment {
            melee: part.radius > 0,
            part: tenchu2::BONES
                .iter()
                .position(|&bone| bone == part.bone)
                .ok_or("Unsupported imported weapon attachment bone")?,
            slot: [0, 1, 4, 2, 3][part.slot],
            model,
        });
    }
    Ok(Equipment {
        draw: match kind & 0xf00 {
            0x100 => Draw::Single,
            0x200 => Draw::Dual,
            _ => Draw::Fixed,
        },
        attachments,
    })
}

fn character_action_exists(registry: &Registry, kind: i16, action: i16) -> bool {
    ACTIONS
        .iter()
        .find(|&&(mapped, _)| mapped == action)
        .is_some_and(|&(_, original)| registry.clip(kind, original).is_some())
}

fn combo(action: i16) -> [Option<i16>; 3] {
    match action {
        0x900 => [
            Some(MOT_ATTACK_SLASH2),
            Some(MOT_ATTACK_SLASH2_RIGHT),
            Some(MOT_ATTACK_SLASH2_LEFT),
        ],
        0x901..=0x903 => [Some(MOT_ATTACK_SLASH3); 3],
        0x904 => [Some(MOT_ATTACK_SLASH4); 3],
        0x905 => [Some(MOT_UNARMED_FINISH); 3],
        0x907 => [None, Some(MOT_ATTACK_RIGHT2), Some(MOT_ATTACK_RIGHT2_LEFT)],
        0x90a => [None, Some(MOT_ATTACK_LEFT2_RIGHT), Some(MOT_ATTACK_LEFT2)],
        _ => [None; 3],
    }
}

fn holster(clip: i16) -> Option<[i16; 2]> {
    let frames = match clip {
        0x10c => [9, 27],
        0x4e1 => [4, 7],
        0x10d => [5, 34],
        0x4e2 => [9, 12],
        0x10e => [12, 37],
        0xbc => [5, 31],
        0x4e3 => [10, 14],
        0x10f => [10, 50],
        0xbd => [2, 30],
        0x4e4 => [11, 16],
        0x110 => [12, 47],
        _ => return None,
    };
    Some(frames)
}

#[cfg(test)]
mod tests;

fn proxy_weapon(kind: i16, weapons: &Weapons) -> i16 {
    let names: Vec<_> = weapons
        .parts(kind)
        .filter_map(|p| p.model.as_deref())
        .collect();
    if names.is_empty() {
        return kind::FIST;
    }
    if kind & 0xf00 == 0x200 {
        return kind::KODATI;
    }
    if names
        .iter()
        .any(|name| name.contains("yari") || name.contains("nagi"))
    {
        return kind::YARI;
    }
    kind::NINJA
}
