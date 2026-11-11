use super::*;
use crate::visuals::interpolation::Part as PartId;
use std::collections::BTreeSet;
use tenchu_core::tenchu2::actor::State;

#[derive(Default)]
pub(super) struct Weapons {
    pub live: BTreeMap<PartId, bool>,
    pub hits: BTreeSet<PartId>,
}

impl Weapons {
    pub fn new(session: &super::super::Session, models: &Models) -> Result<Self> {
        let world = &session.world;
        let mut hits = session.contact.melee_hits.clone();
        let player = &world.actors[world.player].actor;
        if (0x920..=0x926).contains(&player.motion.action)
            && player.target.is_some_and(|target| {
                session
                    .contact
                    .wounds
                    .iter()
                    .any(|wound| wound.owner == target)
            })
        {
            hits.insert(world.player);
        }
        let mut weapons = Self::default();
        for (owner, (agent, instance)) in world.actors.iter().zip(&world.instances).enumerate() {
            let actor = &agent.actor;
            let kind = session
                .assets
                .resources
                .registry
                .actor(actor.definition_kind())
                .ok_or("Missing native weapon definition")?
                .weapon;
            for (slot, root) in instance.rig.weapons.iter().enumerate() {
                let Some(root) = root else { continue };
                let name = models
                    .weapon_names
                    .get(root)
                    .ok_or("Missing native weapon instance")?;
                let parts = &session.assets.resources.weapons;
                let melee = parts.parts(kind).any(|part| {
                    part.model.as_deref() == Some(name)
                        && (part.radius > 0
                            || kind < 0
                                && parts.parts(kind).any(|contact| {
                                    contact.model.is_none()
                                        && contact.bone == part.bone
                                        && contact.radius > 0
                                }))
                });
                if !melee {
                    continue;
                }
                for index in 0..session.assets.weapons[name].len() {
                    let key = interpolation::part(
                        instance.rig.root,
                        root + 1 + index,
                        actor.definition_kind(),
                    );
                    weapons
                        .live
                        .insert(key, actor.weapon_drawn() && actor.state == State::Attack);
                    if actor.weapon_drawn() && slot < 3 && hits.contains(&owner) {
                        weapons.hits.insert(key);
                    }
                }
            }
        }
        Ok(weapons)
    }
}
