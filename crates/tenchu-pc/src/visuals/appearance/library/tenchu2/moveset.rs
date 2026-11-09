use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    executable::{Character, MotionRegistration},
    tenchu2::{self, Edition, motions, registry::Registry, weapons::Weapons},
    weapon::{Attack, Contact, Hitbox, ImportedAttack, kind},
};
use tenchu_core::{game::moveset::Imported, ids::*};

// Gaisen actions with an equivalent in T2's standing, crouch, jump and melee
// controllers. Paired kills and item scripts keep their Gaisen registrations.
const ACTIONS: &[(i16, i16)] = &[
    (MOT_NORMAL, 0),
    (MOT_NORMAL_TURN_R, 1),
    (MOT_NORMAL_TURN_L, 2),
    (MOT_MOVE, 3),
    (MOT_MOVE_BACK, 4),
    (MOT_ENGAGE_STANCE, 0x100),
    (MOT_ENGAGE_TURN_R, 0x101),
    (MOT_ENGAGE_TURN_L, 0x102),
    (MOT_CHASE, 0x103),
    (MOT_CHASE_BACK, 0x104),
    (MOT_SQUAT, 0x200),
    (MOT_SQUAT_WALK_F, 0x201),
    (MOT_SQUAT_WALK_B, 0x202),
    (MOT_SQUAT_WALK_R, 0x203),
    (MOT_SQUAT_WALK_L, 0x204),
    (MOT_SQUAT_ROLL_F, 0x210),
    (MOT_SQUAT_ROLL_B, 0x211),
    (MOT_SQUAT_ROLL_R, 0x212),
    (MOT_SQUAT_ROLL_L, 0x213),
    (MOT_SQUAT_BACKFLIP, 0x220),
    (MOT_STICKON, 0x300),
    (MOT_STICKON_SLIDE_L, 0x301),
    (MOT_STICKON_SLIDE_R, 0x302),
    (MOT_JUMP, 0x600),
    (MOT_JUMP_FORWARD, 0x601),
    (MOT_JUMP_BACK, 0x602),
    (MOT_JUMP_RIGHT, 0x603),
    (MOT_JUMP_LEFT, 0x604),
    (MOT_JUMP_WALLKICK, 0x605),
    (MOT_JUMP_RUN, 0x606),
    (MOT_JUMP_FLIP, 0x607),
    (MOT_ATTACK, 0x900),
    (MOT_ATTACK_SLASH2, 0x901),
    (MOT_ATTACK_SLASH2_RIGHT, 0x902),
    (MOT_ATTACK_SLASH2_LEFT, 0x903),
    (MOT_ATTACK_SLASH3, 0x904),
    (MOT_ATTACK_SLASH4, 0x905),
    (MOT_UNARMED_FINISH, 0x906),
    (MOT_ATTACK_RIGHT1, 0x907),
    (MOT_ATTACK_RIGHT2, 0x908),
    (MOT_ATTACK_RIGHT2_LEFT, 0x909),
    (MOT_ATTACK_LEFT1, 0x90a),
    (MOT_ATTACK_LEFT2, 0x90b),
    (MOT_ATTACK_LEFT2_RIGHT, 0x90c),
    (MOT_ATTACK_LUNGE, 0x910),
    (MOT_ATTACK_CROUCH, 0x914),
    (MOT_ATTACK_BACK, 0x915),
];

pub fn hero(name: &str) -> Option<i16> {
    match name {
        "rikimaru" | "rikia" => Some(0),
        "ayame" | "ayamea" => Some(1),
        "tatumaru" | "tatua" | "tatua_m" => Some(2),
        _ => None,
    }
}

pub fn load(disc: &Disc<impl ReadAt>) -> Result<BTreeMap<i16, Arc<Imported>>> {
    let registry = Registry::load(disc)?;
    // Both discs store authored 30 Hz tracks. PAL's 25 Hz load-time patch is
    // inappropriate for Gaisen's 30 Hz controller (as in the Tatsumaru importer).
    let weapons = Weapons::load(disc)?;
    let registry_ref = &registry;
    let clips = motions::collect(
        disc,
        (0..3)
            .flat_map(|kind| {
                ACTIONS
                    .iter()
                    .filter_map(move |&(_, action)| registry_ref.clip(kind, action))
            })
            .collect(),
    )?;
    let mut out = BTreeMap::new();
    for kind in 0..3 {
        let actor = registry.actor(kind).ok_or("Missing Tenchu 2 hero")?;
        let base: i16 = match registry.edition {
            Edition::Japan => 0x3000,
            Edition::Europe => 0x4800,
        } + kind * 0x800;
        let mut character = Character {
            kind: base + kind,
            weapon: [kind::NINJA, kind::KODATI, kind::FIST][kind as usize],
            turn: actor.turn,
            health: actor.health,
            width: 0,
            height: 0,
            model: actor.model.clone(),
            motions: Vec::new(),
        };
        let mut converted = BTreeMap::new();
        let mut attacks = Vec::new();
        for &(action, original) in ACTIONS {
            let Some(id) = registry.clip(kind, original) else {
                continue;
            };
            if !(0..0x800).contains(&id) {
                return Err("Tenchu 2 motion exceeds reserved import range".into());
            }
            let source = clips
                .get(&id)
                .ok_or_else(|| format!("Missing Tenchu 2 motion {id:#x}"))?;
            let mut clip = source.gaisen_pose();
            clip.id = base
                .checked_add(id)
                .ok_or("Tenchu 2 motion ID out of range")?;
            clip.sweep = (-i32::from(source.blend))
                .try_into()
                .map_err(|_| "Tenchu 2 blend out of range")?;
            clip.forward_speed = source.forward_speed;
            clip.side_speed = source.side_speed;
            if original >= 0x900 {
                let source_attack = weapons
                    .attack(id)
                    .ok_or_else(|| format!("Missing Tenchu 2 attack {id:#x}"))?;
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
                    return Err(format!(
                        "Unsupported Tenchu 2 attack contacts {id:#x}: {}",
                        contacts.len()
                    ));
                }
                let mut contacts = contacts.into_iter();
                let combo = combo(original).map(|next| {
                    next.filter(|&next| character_action_exists(&registry, kind, next))
                });
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
        out.insert(
            kind,
            Arc::new(Imported {
                character,
                clips: converted.into_values().collect(),
                attacks,
            }),
        );
    }
    Ok(out)
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
