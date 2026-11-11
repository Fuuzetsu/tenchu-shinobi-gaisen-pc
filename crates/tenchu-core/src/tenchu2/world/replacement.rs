use super::{Assets, World};
use crate::{Result, tenchu2::rig::WeaponMode};

pub struct Replacement {
    pub kind: i16,
    pub definition: i16,
    pub behavior: u16,
}

impl World {
    /// Replace a starting NPC while retaining its authored slot and mission role.
    pub fn replace_actor(
        &mut self,
        index: usize,
        replacement: Replacement,
        assets: &Assets,
    ) -> Result<()> {
        if self.tick != 0 || index == self.player {
            return Err("Actor replacement is only available before a mission starts".into());
        }
        let original = self
            .actors
            .get(index)
            .ok_or("Missing actor to replace")?
            .clone();
        if original.actor.health == 0 || original.actor.max_health <= 0 {
            return Err("Only living or scripted dormant NPCs can be replaced".into());
        }
        let Replacement {
            kind,
            definition,
            mut behavior,
        } = replacement;
        if kind < 0 || assets.resources.registry.actor(kind).is_none() {
            return Err("Choose a native T2 actor controller".into());
        }
        let mut next = self.clone();
        next.collision.allow_expansion();
        let old = &next.instances[index].rig;
        for &joint in &old.joints {
            next.collision.remove(joint);
        }
        let position = original.actor.position;
        let yaw = original.actor.rotation.y;
        let (agent, instance) = next.build_actor(index, kind, definition, position, yaw, assets)?;
        if agent.actor.max_health <= 0 {
            return Err("The replacement has no living actor controller".into());
        }
        next.actors[index] = agent;
        next.instances[index] = instance;
        if original.behavior.patrol.waypoints().next().is_some() {
            behavior = (behavior & !15) | 3;
        }
        next.configure_actor(
            index,
            &tenchu_assets::tenchu2::layout::Actor {
                kind,
                slot: 0,
                behavior,
                position: position.to_array(),
                facing: yaw,
                route: Vec::new(),
            },
            assets,
        )?;
        let agent = &mut next.actors[index];
        agent.slot = original.slot;
        agent.behavior.home = original.behavior.home;
        agent.behavior.patrol = original.behavior.patrol;
        agent.behavior.profile.protected = original.behavior.profile.protected;
        agent.sight.destination = position;
        let actor = &mut agent.actor;
        actor.mission_kind = Some(original.actor.mission_kind());
        actor.previous = original.actor.previous;
        actor.terrain = original.actor.terrain;
        actor.invulnerable = original.actor.invulnerable;
        if original.actor.health < 0 {
            actor.health = original.actor.health;
        }
        if original.actor.flags & 8 != 0 {
            actor.flags |= 8;
            let rig = &mut next.instances[index].rig;
            actor.set_weapon(
                rig,
                index,
                &mut next.collision,
                &assets.resources,
                WeaponMode::Remove,
            )?;
            actor.set_body_collision(rig.joints[0], index, &mut next.collision, false)?;
        }
        if next.script_hidden.contains(&index) {
            next.set_script_visible(index, false, assets)?;
        }
        next.publish(index, assets);
        next.collision.compute(|id| next.scene.world(id))?;
        // Optional selections must not consume the campaign's gameplay RNG.
        next.random = self.random;
        *self = next;
        Ok(())
    }
}
