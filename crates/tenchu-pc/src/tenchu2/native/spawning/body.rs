use crate::{
    tenchu2::native::models,
    visuals::appearance::library::{Body, Library, actor_behavior},
};
use tenchu_assets::{Result, tenchu2::characters::Rig};
use tenchu_core::tenchu2::world::Assets;

pub(super) fn kind(library: &Library, body: &Body) -> Result<i16> {
    if let Some(native) = &body.native {
        return Ok(native.kind);
    }
    if body.resource == "HUMAN/KARASU.MAD" {
        return Ok(0x74);
    }
    let kind = actor_behavior(library, body)?
        .character
        .as_ref()
        .ok_or("This body has no source actor definition")?
        .kind;
    match kind {
        4 => return Ok(0x13),
        5 => return Ok(0x12),
        6 => return Ok(0x10),
        0xa6 | 0xa7 => {
            return Err(format!(
                "{} is a scripted scenery model; it has no native T2 actor controller",
                body.name
            ));
        }
        0xa8 => {
            return Err("ARROW is a projectile model; it has no native T2 actor controller".into());
        }
        _ => {}
    }
    Assets::gaisen_actor_kind(kind).ok_or_else(|| {
        format!(
            "{} has no native T2 actor counterpart ({kind:#x})",
            body.name
        )
    })
}

pub(super) fn install(assets: &mut Assets, body: &Body, kind: i16) -> Result<i16> {
    let name = format!("placed:{}:{kind}", body.id);
    if let Some(actor) = assets
        .resources
        .registry
        .actors
        .iter()
        .find(|a| a.model == name)
    {
        return Ok(actor.kind);
    }
    let mut definition = assets
        .resources
        .registry
        .actor(kind)
        .ok_or("Missing native actor definition")?
        .clone();
    let (rig, sources) = if let Some(native) = &body.native {
        (
            Rig {
                parts: native.rig.parts.clone(),
                vram: native.rig.vram.clone(),
            },
            (0..native.rig.parts.len())
                .map(|i| format!("{}/{i}", body.resource))
                .collect(),
        )
    } else {
        let parts = models::placement_parts(body, &assets.models[&definition.model].parts)?;
        let sources = models::native_sources(body, parts.len());
        (
            Rig {
                parts,
                vram: (*body.vram).clone(),
            },
            sources,
        )
    };
    tenchu_core::motion::Pose::new(&rig.parts)?;
    definition.kind = assets
        .resources
        .registry
        .actors
        .iter()
        .map(|a| a.kind)
        .min()
        .unwrap_or(-1)
        .min(-1)
        .checked_sub(1)
        .ok_or("Too many imported actor models")?;
    definition.model.clone_from(&name);
    let id = definition.kind;
    assets.model_sources.insert(name.clone(), sources);
    assets.models.insert(name, rig);
    assets.resources.registry.actors.push(definition);
    Ok(id)
}
