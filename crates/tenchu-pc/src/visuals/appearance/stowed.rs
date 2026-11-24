use super::*;
use tenchu_assets::weapon::{Attachment, Draw};
use tenchu_core::{actor::State, ids::*};

pub(super) fn unarmed_kill(player: &tenchu_core::actor::Actor) -> bool {
    player.health > 0
        && player.status == State::Attack
        && player.target.is_some()
        && player.attribute & ATTR_WEAPON_DRAWN != 0
        && matches!((player.kind, player.motion.action), (0, 0x716) | (1, 0x719))
}

pub(super) fn visit(
    game: &Game,
    sheath: Option<&Attachment>,
    matrices: &[Matrix],
    inspection: bool,
    include: impl Fn(Part) -> bool,
    mut visit: impl FnMut(Part, &[model::Geometry], Matrix),
) {
    let player = game.player();
    let mut slots = player.weapons.each_ref().map(Option::as_ref);
    let draw = player
        .imported_weapon_draw
        .unwrap_or_else(|| Draw::gaisen(game.mission.characters[player.definition].weapon));
    draw.swap(&mut slots);
    for weapon in [slots[0], slots[1], slots[4]].into_iter().flatten() {
        let key = Part::Actor {
            root: player.root_model,
            model: weapon.model,
            kind: weapon.kind,
            inspection,
        };
        let local = &game.scene.models[weapon.model];
        let Some(bone) = player.parts.iter().position(|&p| Some(p) == local.parent) else {
            continue;
        };
        // Fixed equipment has no holster model. Leave its body accessories in place.
        if matches!(bone, 13 | 14) || !include(key) {
            continue;
        }
        let mut matrix = matrices[bone].compose(&local.local);
        if let Some(sheath) = sheath
            && let Some(source) = game.mission.weapons.imported_models.get(&weapon.kind)
        {
            if matches!(
                source.source_id.as_str(),
                "tenchu2/weapons/ninB/0" | "tenchu2/weapons/ninC/0"
            ) {
                matrix.translation = matrix
                    .translation
                    .wrapping_add(matrices[1].apply_large([0, 0, 60].into()));
            } else if source.source_id == "weapons/0013/grip" {
                matrix = matrices[1].compose(&Matrix {
                    translation: sheath::grip_mount(sheath.position).translation,
                    ..Default::default()
                });
            }
        }
        visit(key, &game.mission.weapons.models[&weapon.kind], matrix);
    }
    if player.imported_weapon_draw.is_some() {
        return;
    }
    for (slot, weapon) in slots.iter().enumerate().take(4).skip(2) {
        let Some(weapon) = weapon else { continue };
        let Some(geometry) = game.mission.weapons.grips.get(&weapon.kind) else {
            continue;
        };
        let Some((bone, mount)) = crate::sheathing::stowed_transform(player.kind, slot) else {
            continue;
        };
        let key = Part::Actor {
            root: player.root_model,
            model: weapon.model,
            kind: weapon.kind,
            inspection,
        };
        let mount = sheath
            .filter(|_| slot == 2 && player.kind == 0)
            .map_or(mount, |s| sheath::grip_mount(s.position));
        if include(key) {
            visit(key, geometry, matrices[bone].compose(&mount));
        }
    }
}

#[cfg(test)]
mod tests;
