use super::*;
use crate::visuals::appearance::cast::{Actor, Cast};

pub(super) fn prepare(session: &super::super::super::Session, graphics: &Graphics) -> Result<Cast> {
    let director = &session
        .gaisen
        .as_ref()
        .ok_or("Missing cinematic director")?
        .director;
    let game = &director.source;
    let mut bodies = Vec::new();
    let mut names = BTreeMap::new();
    let mut weapons = BTreeMap::new();
    let mut actors = BTreeMap::new();
    for &id in &game.roster.order {
        if id == game.player_id {
            continue;
        }
        let source = game.roster.get(id);
        let Some(index) = director.native_actor(id) else {
            continue;
        };
        let native = &session.world.actors[index].actor;
        let definition = session
            .assets
            .resources
            .registry
            .actor(native.definition_kind())
            .ok_or("Missing cinematic cast definition")?;
        let rig = &session.assets.models[&definition.model];
        if source.parts.len() != tenchu_assets::tenchu2::BONES.len()
            || rig.parts.len() < tenchu_assets::tenchu2::BONES.len()
        {
            continue;
        }
        let body = if let Some(&body) = names.get(&definition.model) {
            body
        } else {
            let index = bodies.len();
            bodies.push(player::body(session, &definition.model, graphics)?);
            names.insert(definition.model.clone(), index);
            index
        };
        if let std::collections::btree_map::Entry::Vacant(entry) = weapons.entry(definition.weapon)
        {
            entry.insert(player::weapons(session, definition.weapon)?);
        }
        actors.insert(
            source.root_model,
            Actor {
                body,
                equipment: weapons[&definition.weapon].clone(),
            },
        );
    }
    Ok(Cast::new(bodies, actors))
}
