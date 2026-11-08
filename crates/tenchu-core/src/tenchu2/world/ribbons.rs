use super::{Assets, World};
use crate::{
    Result,
    tenchu2::{
        actor::State,
        effects::{
            Effect,
            ribbon::{Sample, Source},
        },
    },
};

impl World {
    pub(super) fn sample_weapon_ribbons(&mut self, assets: &Assets) -> Result<()> {
        for effect in &mut self.effects.entries {
            let Effect::Ribbon(ribbon) = effect else {
                continue;
            };
            let Source::Weapon { actor, slot, part } = ribbon.source else {
                continue;
            };
            if !ribbon.active() {
                continue;
            }
            let agent = &self.actors[actor].actor;
            if agent.attack.trails & (1 << slot) == 0 || agent.flags & 8 != 0 {
                ribbon.push(None);
                continue;
            }
            if agent.state != State::Attack || agent.flags & 0x100000 == 0 {
                continue;
            }
            let part = &assets.resources.weapons.parts[part];
            let bones = agent
                .attack
                .index
                .map_or(0, |i| assets.resources.weapons.attacks[i].bones);
            if bones & (1 << part.bone) == 0 {
                continue;
            }
            let matrix = self.camera.view.compose(
                &self
                    .scene
                    .world(self.instances[actor].rig.joints[part.bone])?,
            );
            ribbon.push(Some(Sample::weapon(&matrix, part.trail)));
        }
        Ok(())
    }
}
