use super::*;
use tenchu_core::tenchu2::{actor::Actor, world::Instance};

pub(super) fn unarmed_kill(actor: &Actor, assets: &Assets) -> bool {
    actor.health > 0
        && actor.state == tenchu_core::tenchu2::actor::State::Attack
        && actor.target.is_some()
        && actor.motion.action == 0x926
        && assets.resources.registry.clip(actor.kind, 0x926) == actor.motion.clip
        && match actor.kind {
            // Source clip120's unarmed impacts are16/41/66; its later throw
            // prepares the blade finish (blood at130). Clip158 uses only legs.
            0 => actor.motion.frame <= 66,
            1 => true,
            _ => false,
        }
}

pub(super) fn mount(
    model: usize,
    instance: &Instance,
    scene: &tenchu_core::scene::Scene,
    matrices: &[Matrix],
) -> Result<Matrix> {
    let mut path = vec![scene.models[model].local];
    let mut parent = scene.models[model].parent;
    while let Some(id) = parent {
        if let Some(bone) = instance.rig.joints.iter().position(|&j| j == id) {
            return Ok(path
                .iter()
                .rev()
                .fold(matrices[bone], |matrix, local| matrix.compose(local)));
        }
        let model = &scene.models[id];
        path.push(model.local);
        parent = model.parent;
    }
    Err("Missing stowed weapon joint".into())
}
