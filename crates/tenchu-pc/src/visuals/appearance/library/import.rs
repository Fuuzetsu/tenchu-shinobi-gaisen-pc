use super::*;
use tenchu_core::ids::MOT_NORMAL;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Imported {
    pub actor: tenchu_assets::stage::imports::Actor,
    pub voice: Option<i16>,
    pub think: i16,
}

pub fn import_actor(
    mission: &mut MissionAssets,
    library: &Library,
    id: &str,
    imported: &mut BTreeMap<String, Imported>,
) -> Result<Imported> {
    let body = library.get(id).ok_or("Unknown actor model")?;
    if let Some(&source) = imported.get(&body.id) {
        return Ok(source);
    }
    let behavior = if body.character.is_some() {
        body
    } else {
        library
            .bodies
            .iter()
            .filter(|b| {
                b.character
                    .as_ref()
                    .is_some_and(|c| c.kind & 0xf0 == 0xa0 && c.health > 0)
            })
            .min_by_key(|b| {
                (
                    b.parts.len().abs_diff(body.parts.len()),
                    b.character.as_ref().unwrap().health,
                )
            })
            .ok_or("No animal behavior is available for this resource-only model")?
    };
    let mut character = behavior.character.clone().unwrap();
    if !character.motions.iter().any(|r| r.action == MOT_NORMAL)
        && let Some(idle) = behavior.idle
    {
        character.motions.push(MotionRegistration {
            action: MOT_NORMAL,
            clip: behavior.clips[idle].id,
        });
    }
    character.health = character.health.max(1);
    character.width = character.width.max(1);
    character.turn = character.turn.max(1);
    let actor = mission.import_actor(
        character,
        body.parts.clone(),
        &behavior.clips,
        body.vram.clone(),
    )?;
    let voice = behavior.voice.map(|voice| {
        let program = 128 + mission.imported_voices.len() as i16;
        mission.imported_voices.push(voice);
        program
    });
    let source = Imported {
        actor,
        voice,
        think: behavior.think,
    };
    imported.insert(body.id.clone(), source);
    Ok(source)
}
