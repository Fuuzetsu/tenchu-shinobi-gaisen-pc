use super::story::Story;
use crate::visuals::interpolation::{Part, Snapshot};
use std::collections::BTreeMap;
use tenchu_assets::Result;
use tenchu_core::tenchu2::{
    actor::Actor,
    world::{Assets, Instance, World},
};

pub(super) fn part(root: usize, model: usize, kind: i16) -> Part {
    Part::Actor {
        root,
        model,
        kind,
        inspection: false,
    }
}

pub(super) fn weapons<'a>(
    actor: &Actor,
    instance: &Instance,
    assets: &'a Assets,
) -> Result<impl Iterator<Item = (usize, &'a str)>> {
    let definition = assets
        .resources
        .registry
        .actor(actor.definition_kind())
        .ok_or("Missing native actor definition")?;
    let mut roots: Vec<_> = instance.rig.weapons.iter().flatten().copied().collect();
    roots.sort_unstable();
    Ok(roots.into_iter().zip(
        assets
            .resources
            .weapons
            .parts(definition.weapon)
            .filter_map(|part| part.model.as_deref()),
    ))
}

pub(super) fn gameplay(world: &World, assets: &Assets) -> Result<Snapshot> {
    let mut scene = world.scene.clone();
    let mut parts = BTreeMap::new();
    let mut roots = BTreeMap::new();
    for (agent, instance) in world.actors.iter().zip(&world.instances) {
        let actor = &agent.actor;
        if actor.flags & 8 != 0 {
            continue;
        }
        let root = instance.rig.root;
        let kind = actor.definition_kind();
        roots.insert(root, actor.position);
        for &joint in &instance.rig.joints {
            parts.insert(part(root, joint, kind), scene.world(joint)?);
        }
        for (weapon, name) in weapons(actor, instance, assets)? {
            if instance.rig.weapons[..3].contains(&Some(weapon)) {
                for index in 0..assets.weapons[name].len() {
                    let joint = weapon + 1 + index;
                    parts.insert(part(root, joint, kind), scene.world(joint)?);
                }
            }
        }
    }
    Ok(Snapshot {
        tick: u64::from(world.tick),
        parts,
        roots,
        view: world.camera.view,
        cinematic: (false, world.sequence as i16),
        camera_mode: world.camera.mode as u32,
        death_blood: None,
    })
}

pub(super) fn story(story: &Story) -> Result<Snapshot> {
    let mut parts = BTreeMap::new();
    let mut roots = BTreeMap::new();
    for (slot, (motion, animation)) in story
        .state
        .actors
        .iter()
        .zip(&story.resources.script.actors)
        .enumerate()
    {
        let Some(matrices) = story.matrices(slot)? else {
            continue;
        };
        roots.insert(slot, motion.position);
        for (joint, matrix) in matrices.into_iter().enumerate() {
            parts.insert(part(slot, joint, animation.model), matrix);
        }
    }
    Ok(Snapshot {
        tick: story.state.frame.max(0) as u64,
        parts,
        roots,
        view: story.state.view,
        cinematic: (true, story.event as i16),
        camera_mode: 0,
        death_blood: None,
    })
}

#[cfg(test)]
mod tests;
