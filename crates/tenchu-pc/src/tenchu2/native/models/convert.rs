use super::*;

pub(super) fn native_parts(body: &Body, original: &[Part]) -> Result<Vec<Part>> {
    if body.parts.len() != BONES.len() {
        return Err("This model has no compatible humanoid rig".into());
    }
    let parents = crate::visuals::appearance::parents(&body.parts);
    let mut parts = original.to_vec();
    if parts.len() < 15 {
        return Err("Host player has no compatible humanoid rig".into());
    }
    for (i, &bone) in BONES.iter().enumerate() {
        let mut part = body.parts[i].clone();
        part.child = bone as i16;
        part.parent = parents[i].map_or(-1, |p| BONES[p] as i16);
        for geometry in &mut part.geometry {
            for joint in &mut geometry.joints {
                *joint = BONES[usize::from(*joint)] as u8;
            }
        }
        parts[bone] = part;
    }
    for part in parts.iter_mut().skip(15) {
        part.geometry.clear();
    }
    Ok(parts)
}

pub(super) fn native_sources(body: &Body, count: usize) -> Vec<String> {
    let mut out = vec![String::new(); count];
    for (&bone, source) in BONES.iter().zip(body.sources()) {
        out[bone] = source.id;
    }
    out
}

pub(super) fn convert_moves(
    body: &Body,
    donor: &Game,
    definition: &mut registry::Actor,
    assets: &mut Assets,
    options: Options,
) -> Result<BTreeMap<i16, ImportedAttack>> {
    let character = body.moveset_character().ok_or("Model has no moveset")?;
    let clips = body
        .imported_moveset
        .as_ref()
        .map_or(body.clips.as_slice(), |m| m.clips.as_slice());
    let weapon_kind = donor.mission.characters[donor.player().definition].weapon;
    if !options.player_movement {
        definition.turn = character.turn;
    }
    assets.resources.clips.retain(|&id, _| id >= 0);
    assets.resources.weapons.attacks.retain(|a| a.clip >= 0);
    let mut attacks = BTreeMap::new();
    let mut mapped = BTreeMap::new();
    let owns_attack = character
        .motions
        .iter()
        .any(|m| m.action == tenchu_core::ids::MOT_ATTACK);
    if owns_attack {
        definition.weapon = if let Some(source) = &body.imported_moveset {
            match source.equipment.draw {
                tenchu_assets::weapon::Draw::Fixed => i16::MIN,
                tenchu_assets::weapon::Draw::Single => i16::MIN + 0x100,
                tenchu_assets::weapon::Draw::Dual => i16::MIN + 0x200,
            }
        } else {
            match weapon_kind {
                kind::KODATI | kind::JYUTE | kind::EN | kind::KATANA_2 => i16::MIN + 0x200,
                kind::KOZUKA
                | kind::NINJA
                | kind::KEITOU
                | kind::KATANA_0
                | kind::HOUTOU
                | kind::KATANA_1 => i16::MIN + 0x100,
                _ => i16::MIN,
            }
        };
        definition
            .motions
            .retain(|r| !(0x900..0x917).contains(&r.action));
    }
    for &(gaisen, native) in T2_ACTIONS {
        if options.player_movement && native < 0x900 && !matches!(native, 0 | 0x100) {
            continue;
        }
        let Some(registration) = character.motions.iter().find(|m| m.action == gaisen) else {
            continue;
        };
        let Some(clip) = clips.iter().find(|c| c.id == registration.clip) else {
            continue;
        };
        let id = -2 - mapped.len() as i16;
        let id = *mapped.entry(clip.id).or_insert(id);
        let mut bones = vec![
            vec![tenchu_assets::motion::Key {
                time: 0,
                value: [0; 3]
            }];
            15
        ];
        for (&bone, keys) in BONES.iter().zip(&clip.bones) {
            bones[bone].clone_from(keys);
        }
        assets.resources.clips.insert(
            id,
            motions::Clip {
                id,
                duration: clip.duration,
                blend: i16::from(clip.sweep).saturating_neg().max(0),
                forward_speed: clip.forward_speed,
                side_speed: clip.side_speed,
                root: clip.root.clone(),
                bones,
            },
        );
        if native >= 0x900 {
            let Some(attack) = donor
                .mission
                .weapons
                .attacks
                .iter()
                .find(|a| a.clip == clip.id)
            else {
                continue;
            };
            if attack.start <= 0 {
                continue;
            }
            let mut metadata = if let Some(imported) =
                donor.mission.weapons.imported_attacks.get(&clip.id)
            {
                imported.clone()
            } else {
                let parts = match weapon_kind {
                    kind::FIST => [8, 11],
                    kind::JAW => [2, 1],
                    _ => [13, 14],
                };
                let contacts = std::array::from_fn(|i| {
                    donor.player().weapon_hitbox[i]
                        .and_then(|index| donor.mission.weapons.hitboxes.get(index))
                        .map(|h| Contact {
                            part: parts[i],
                            hitbox: h.clone(),
                        })
                });
                ImportedAttack {
                    contacts,
                    combo: native_combo(native).map(|a| {
                        a.and_then(|a| T2_ACTIONS.iter().find(|&&(_, n)| n == a).map(|&(g, _)| g))
                    }),
                    turn_on_exit: true,
                    holster: None,
                }
            };
            for contact in metadata.contacts.iter_mut().flatten() {
                contact.part = BONES[contact.part];
            }
            for action in metadata.combo.iter_mut().flatten() {
                *action = T2_ACTIONS
                    .iter()
                    .find(|&&(g, _)| g == *action)
                    .ok_or("Unmapped combo")?
                    .1;
            }
            let bones = metadata
                .contacts
                .iter()
                .flatten()
                .fold(0, |mask, c| mask | (1 << c.part));
            assets.resources.weapons.attacks.push(weapons::Attack {
                clip: id,
                power: attack.power.clamp(0, 255) as u8,
                steering: attack.steering.clamp(0, 255) as u8,
                combo: attack.combo.clamp(0, 255) as u8,
                start: attack.start.clamp(0, 255) as u8,
                end: attack.end.clamp(0, 255) as u8,
                trail_start: attack.trail_start.clamp(0, 255) as u8,
                trail_end: attack.trail_end.clamp(0, 255) as u8,
                bones,
            });
            attacks.insert(id, metadata);
        }
        definition.motions.retain(|r| r.action != native);
        definition.motions.push(registry::Registration {
            action: native,
            clip: id,
        });
    }
    let supported: BTreeSet<_> = definition.motions.iter().map(|m| m.action).collect();
    for attack in attacks.values_mut() {
        for next in &mut attack.combo {
            *next = next.filter(|action| supported.contains(action));
        }
    }
    Ok(attacks)
}

fn native_combo(action: i16) -> [Option<i16>; 3] {
    match action {
        0x900 => [Some(0x901), Some(0x902), Some(0x903)],
        0x901 => [Some(0x904); 3],
        0x904 => [Some(0x905); 3],
        0x907 => [None, Some(0x908), Some(0x909)],
        0x90a => [None, Some(0x90c), Some(0x90b)],
        _ => [None; 3],
    }
}

pub(super) fn convert_weapons(
    donor: &Game,
    kind: i16,
    attacks: &BTreeMap<i16, ImportedAttack>,
    assets: &mut Assets,
) -> Result<Vec<weapons::Part>> {
    assets
        .weapons
        .retain(|name, _| !assets.weapon_imports.contains_key(name));
    assets.weapon_imports.clear();
    let actor = donor.player();
    let mut slots = actor.weapons.clone();
    if actor.attribute & tenchu_core::ids::ATTR_WEAPON_DRAWN != 0 {
        if let Some(draw) = actor.imported_weapon_draw {
            draw.swap(&mut slots);
        } else {
            if kind & 0xf00 == 0x200 {
                slots.swap(1, 3);
            }
            if matches!(kind & 0xf00, 0x100 | 0x200) {
                slots.swap(0, 2);
            }
        }
    }
    let mut parts = Vec::new();
    let vram = std::sync::Arc::new(donor.mission.vram.clone());
    for (slot, weapon) in slots
        .iter()
        .enumerate()
        .filter_map(|(i, w)| w.as_ref().map(|w| (i, w)))
    {
        let model = &donor.scene.models[weapon.model];
        let bone = actor
            .parts
            .iter()
            .position(|&p| Some(p) == model.parent)
            .ok_or("Missing Gaisen weapon bone")?;
        let name = format!("imported-weapon-{}", weapon.kind);
        let source = donor
            .mission
            .weapons
            .imported_models
            .get(&weapon.kind)
            .cloned()
            .unwrap_or_else(|| {
                std::sync::Arc::new(tenchu_assets::weapon::ImportedModel {
                    id: format!("gaisen:weapons/{}", weapon.kind),
                    source_id: format!("weapons/{:04x}", weapon.kind),
                    geometry: donor.mission.weapons.models[&weapon.kind].clone(),
                    vram: vram.clone(),
                })
            });
        assets.weapons.insert(
            name.clone(),
            vec![Part {
                parent: -1,
                child: 0,
                position: model.local.translation.to_array().map(|v| v as i16),
                geometry: donor.mission.weapons.models[&weapon.kind].clone(),
            }],
        );
        assets.weapon_imports.insert(name.clone(), source);
        parts.push(weapons::Part {
            kind,
            model: Some(name),
            bone: BONES[bone],
            slot: [0, 1, 3, 4, 2][slot],
            trail: [[0; 3]; 2],
            center: [0; 3],
            radius: 0,
        });
    }
    let mut contacts = BTreeMap::new();
    for contact in attacks.values().flat_map(|a| a.contacts.iter().flatten()) {
        contacts.entry(contact.part).or_insert(&contact.hitbox);
    }
    parts.extend(contacts.into_iter().map(|(bone, h)| weapons::Part {
        kind,
        bone,
        model: None,
        slot: 0,
        trail: h.trail,
        center: h.center,
        radius: h.radius,
    }));
    assets.resources.weapons.parts.retain(|p| p.kind >= 0);
    assets.resources.weapons.parts.extend(parts.iter().cloned());
    Ok(parts)
}
