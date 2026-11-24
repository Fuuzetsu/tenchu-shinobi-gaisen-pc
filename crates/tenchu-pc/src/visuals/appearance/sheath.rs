use super::library::Library;
use tenchu_assets::{model, weapon::Attachment};
use tenchu_core::game::Game;

pub(super) fn replacement(game: &Game, selection: &str, library: &Library) -> Option<Attachment> {
    let resource = if selection.is_empty() {
        if game.player().rendered_mesh_kind != 0 {
            return None;
        }
        game.mission
            .characters
            .iter()
            .find(|c| c.kind == 0)?
            .model
            .as_str()
    } else {
        library
            .get(selection)?
            .resource
            .strip_prefix("HUMAN/")?
            .strip_suffix(".MAD")?
    };
    if !matches!(resource, "RIKIMARU" | "RIKIMAUA") {
        return None;
    }
    ["tenchu2-jp:rikimaru", "tenchu2:rikimaru"]
        .into_iter()
        .filter_map(|id| library.get(id)?.equipment.as_ref())
        .flat_map(|equipment| &equipment.attachments)
        .find(|part| part.slot == 4 && part.part == 1 && !part.melee)
        .cloned()
        .map(|mut part| {
            // T1's back is thicker than the T2 torso the accessory was authored for.
            part.position[2] += 60;
            part
        })
}

pub(super) fn equipped(game: &Game) -> Option<Attachment> {
    let weapon = game.player().weapons[4].as_ref()?;
    let model = game.mission.weapons.imported_models.get(&weapon.kind)?;
    if model.source_id != "tenchu2/weapons/ninC/0" {
        return None;
    }
    Some(Attachment {
        part: 1,
        slot: 4,
        position: [0, 0, 60],
        melee: false,
        model: model.clone(),
    })
}

pub(super) fn grip_mount(position: [i16; 3]) -> tenchu_core::math::Matrix {
    let (_, mut mount) = crate::sheathing::stowed_transform(0, 2).unwrap();
    let guard = mount.apply_large([1, 70, -82].into());
    mount.translation =
        std::array::from_fn(|i| [-144, -439, 149][i] + i32::from(position[i]) - guard[i]).into();
    mount
}

pub(super) fn remove_original(parts: &mut [model::Part]) -> bool {
    parts.get_mut(1).is_some_and(remove_torso)
}

pub(crate) fn remove_torso(part: &mut model::Part) -> bool {
    let Some(torso) = part.geometry.first_mut() else {
        return false;
    };
    // RIKIMAUA has a continuous armoured back and no built-in scabbard.
    if torso.vertices.len() == 24 && torso.faces.len() == 30 {
        return true;
    }
    if torso.vertices.len() != 37 || torso.faces.len() != 65 {
        return false;
    }
    // RIKIMARU's scabbard shares its boundary with the back. Close that opening
    // using the neighbouring cloth's original corners, colours and UVs.
    let mut patch = torso.faces[45].clone();
    let corners = [
        torso.faces[44].corners[1],
        torso.faces[44].corners[0],
        torso.faces[45].corners[1],
        torso.faces[45].corners[2],
    ];
    let mut index = 0;
    torso.faces.retain(|_| {
        let keep = !matches!(index, 46 | 47 | 50 | 51 | 54..=64);
        index += 1;
        keep
    });
    for order in [[0, 1, 2], [2, 1, 3]] {
        patch.corners = order.map(|i| corners[i]).to_vec();
        torso.faces.push(patch.clone());
    }
    true
}

#[cfg(test)]
mod tests;
