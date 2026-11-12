use crate::{
    settings::Graphics,
    tenchu2::native::Session,
    visuals::{
        appearance::{self, Appearance},
        catalog::{ModelSource, Role},
    },
};
use std::sync::Arc;
use tenchu_assets::{
    Result,
    tenchu2::BONES,
    weapon::{Attachment, Draw, Equipment, ImportedModel},
};

pub(super) fn appearance(session: &Session, graphics: &Graphics) -> Result<Option<Appearance>> {
    if let Some(custom) = &session.models.custom {
        return Appearance::custom(
            &custom.id,
            &custom.pack,
            &session.assets.sprites.vram,
            graphics,
        )
        .map(Some);
    }
    let Some(body) = session.assets.models.get("override-player") else {
        return Ok(None);
    };
    let parents = appearance::parents(&body.parts);
    let mut parts = Vec::with_capacity(BONES.len());
    for (index, &bone) in BONES.iter().enumerate() {
        let mut part = body.parts[bone].clone();
        part.child = index as i16;
        part.parent = parents[bone]
            .and_then(|p| BONES.iter().position(|&b| b == p))
            .map_or(-1, |p| p as i16);
        for geometry in &mut part.geometry {
            for joint in &mut geometry.joints {
                *joint = BONES
                    .iter()
                    .position(|&bone| bone == usize::from(*joint))
                    .ok_or("Imported cinematic body has an unmapped joint")?
                    as u8;
            }
        }
        parts.push(part);
    }
    let ids = &session.assets.model_sources["override-player"];
    let sources = parts
        .iter()
        .zip(BONES)
        .map(|(part, bone)| ModelSource {
            id: ids[bone].clone(),
            role: Role::Actor,
            hero_part: crate::visuals::heroes::part(&ids[bone]),
            label: String::new(),
            geometry: &part.geometry,
        })
        .collect();
    let meshes = super::super::build(&body.vram, sources, graphics)?;
    Appearance::from_parts(parts, meshes).map(Some)
}

pub(super) fn equipment(session: &Session) -> Result<Option<Equipment>> {
    if !session.models.has_player_override() {
        return Ok(None);
    }
    let assets = &session.assets;
    let player = &session.world.actors[session.world.player].actor;
    let kind = assets
        .resources
        .registry
        .actor(player.definition_kind())
        .ok_or("Missing imported cinematic player definition")?
        .weapon;
    let vram = Arc::new(assets.weapon_vram.clone());
    let mut attachments = Vec::new();
    for attachment in assets.resources.weapons.parts(kind) {
        let Some(name) = &attachment.model else {
            continue;
        };
        let parts = &assets.weapons[name];
        if parts.len() != 1 || parts[0].parent != -1 {
            return Err(format!("Unsupported articulated cinematic weapon: {name}"));
        }
        let model = assets.weapon_imports.get(name).cloned().unwrap_or_else(|| {
            Arc::new(ImportedModel {
                id: format!(
                    "{}weapons/{name}",
                    assets.resources.weapons.edition.model_prefix()
                ),
                source_id: format!("tenchu2/weapons/{name}/0"),
                geometry: parts[0].geometry.clone(),
                vram: vram.clone(),
            })
        });
        attachments.push(Attachment {
            part: BONES
                .iter()
                .position(|&bone| bone == attachment.bone)
                .ok_or("Unsupported cinematic weapon attachment bone")?,
            position: parts[0].position,
            slot: [0, 1, 4, 2, 3][attachment.slot],
            melee: attachment.radius > 0,
            model,
        });
    }
    Ok(Some(Equipment {
        draw: match kind & 0xf00 {
            0x100 => Draw::Single,
            0x200 => Draw::Dual,
            _ => Draw::Fixed,
        },
        attachments,
    }))
}
