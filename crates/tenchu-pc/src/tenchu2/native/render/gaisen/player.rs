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
    if !session.assets.models.contains_key("override-player") {
        return Ok(None);
    }
    body(session, "override-player", graphics).map(Some)
}

pub(super) fn body(session: &Session, name: &str, graphics: &Graphics) -> Result<Appearance> {
    let body = &session.assets.models[name];
    let parts = remap_parts(&body.parts)?;
    let ids = session.assets.model_sources.get(name);
    let sources = parts
        .iter()
        .zip(BONES)
        .map(|(part, bone)| {
            let id = ids.map_or_else(
                || format!("tenchu2/actors/{name}/{bone}"),
                |ids| ids[bone].clone(),
            );
            ModelSource {
                hero_part: crate::visuals::heroes::part(&id),
                id,
                role: Role::Actor,
                label: String::new(),
                geometry: &part.geometry,
            }
        })
        .collect();
    let meshes = super::super::build(&body.vram, sources, graphics)?;
    Appearance::from_parts(parts, meshes)
}

fn remap_parts(source: &[tenchu_assets::model::Part]) -> Result<Vec<tenchu_assets::model::Part>> {
    tenchu_core::motion::Pose::new(source)?;
    let parents = appearance::parents(source);
    let mut roots = Vec::with_capacity(source.len());
    let mut mapped = Vec::with_capacity(source.len());
    for index in 0..source.len() {
        let mut root = [0; 3];
        let mut ancestor = Some(index);
        let mut target = None;
        while let Some(bone) = ancestor {
            for (value, offset) in root.iter_mut().zip(source[bone].position) {
                *value += i32::from(offset);
            }
            target = target.or_else(|| BONES.iter().position(|&b| b == bone));
            ancestor = parents[bone];
        }
        roots.push(root);
        mapped.push(target.ok_or("Cinematic body has an unmapped root")?);
    }
    let mut parts = Vec::with_capacity(BONES.len());
    for (index, &bone) in BONES.iter().enumerate() {
        let mut part = source
            .get(bone)
            .ok_or("Incomplete cinematic body rig")?
            .clone();
        let parent = parents[bone].map(|p| mapped[p]);
        part.child = index as i16;
        part.parent = parent.map_or(-1, |p| p as i16);
        for (axis, value) in part.position.iter_mut().enumerate() {
            *value = (roots[bone][axis] - parent.map_or(0, |p| roots[BONES[p]][axis]))
                .try_into()
                .map_err(|_| "Cinematic joint position overflow")?;
        }
        part.geometry.clear();
        parts.push(part);
    }
    // Match the T2 importer: costume joints follow their nearest retained bone.
    for (bone, source) in source.iter().enumerate() {
        let owner = mapped[bone];
        for geometry in &source.geometry {
            let mut geometry = geometry.clone();
            for vertex in &mut geometry.vertices {
                for (axis, value) in vertex.iter_mut().enumerate() {
                    *value = (i32::from(*value) + roots[bone][axis] - roots[BONES[owner]][axis])
                        .try_into()
                        .map_err(|_| "Cinematic vertex position overflow")?;
                }
            }
            for joint in &mut geometry.joints {
                *joint = *mapped
                    .get(usize::from(*joint))
                    .ok_or("Invalid cinematic joint")? as u8;
            }
            parts[owner].geometry.push(geometry);
        }
    }
    Ok(parts)
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
    weapons(session, kind).map(Some)
}

pub(super) fn weapons(session: &Session, kind: i16) -> Result<Equipment> {
    let assets = &session.assets;
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
    Ok(Equipment {
        draw: match kind & 0xf00 {
            0x100 => Draw::Single,
            0x200 => Draw::Dual,
            _ => Draw::Fixed,
        },
        attachments,
    })
}
