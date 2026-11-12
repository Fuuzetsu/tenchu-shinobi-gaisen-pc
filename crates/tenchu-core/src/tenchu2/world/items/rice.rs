use super::*;
use crate::tenchu2::actor::flags::{BURNING, POISONED};
use crate::tenchu2::{
    ai::bait::Bait,
    items::rice,
    terrain::{self, Query},
};

impl World {
    pub(super) fn update_rice(
        &mut self,
        index: usize,
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<bool> {
        let entry = &mut self.items.entries[index];
        let State::Rice(rice) = &mut entry.state else {
            unreachable!()
        };
        let model = entry.model;
        let position = rice.flight.position;
        for agent in &mut self.actors {
            if let Some(bait) = &mut agent.behavior.bait
                && bait.model == model
            {
                bait.position = position;
            }
        }
        let phase = (u32::from(self.items.pool_phase) + 2 + u32::from(entry.slot) * 37) & 15;
        if self.tick & 15 == phase {
            if let Some(target) = rice.target {
                let agent = &mut self.actors[target];
                if agent.actor.state == ActorState::Dead || agent.actor.flags & 3 != 0 {
                    agent.behavior.release_bait(&mut agent.actor, self.player);
                    rice.target = None;
                }
            } else if terrain::level(&self.terrain, &mut self.cursor, position, Query::Surface)?
                .is_some()
                && self.cursor.current.is_some_and(|s| {
                    self.terrain.cells[s.cell].nodes[s.node].attributes & 0x200 == 0
                })
            {
                rice.target = rice::nearest(
                    rice.owner,
                    position,
                    &self.actors,
                    &self.terrain,
                    &mut self.cursor,
                    &assets.resources.math,
                )?;
                if let Some(target) = rice.target {
                    let agent = &mut self.actors[target];
                    if agent.actor.state != ActorState::Dead && agent.actor.flags & 3 == 0 {
                        agent.behavior.bait = Some(Bait { model, position });
                        agent.behavior.reset(false);
                        agent.actor.target = None;
                    }
                }
            }
        }
        let contacts: Vec<_> = self.collision.contacts(model).collect();
        let mut alive = true;
        let mut recovered = Vec::new();
        for contact in contacts {
            let slot = self
                .collision
                .slot(contact.index)
                .ok_or("Missing Tenchu 2 rice contact")?;
            if slot.flags & BODY == 0 {
                if slot.flags & 0x200 == 0 {
                    rice.flight.speed = 0;
                }
                continue;
            }
            if let Some(target) = rice.target {
                let agent = &mut self.actors[target];
                agent.behavior.release_bait(&mut agent.actor, self.player);
            }
            let Some(taker) = slot.owner else {
                continue;
            };
            let actor = &mut self.actors[taker].actor;
            if actor.flags & 0xf00 != 0
                || !matches!(
                    actor.state,
                    ActorState::Normal
                        | ActorState::Ready
                        | ActorState::Crouched
                        | ActorState::Wall
                        | ActorState::Ledge
                        | ActorState::Swimming
                )
                || actor.motion.action == 0x513
                || (actor.flags & 3 != 0 && taker != rice.owner)
            {
                rice.flight.speed = 0;
                continue;
            }
            if actor.flags & 0x1000 == 0 && !(0x72..=0x75).contains(&actor.kind) {
                let action = if actor.kind & 0xf8 == 0x78 {
                    0x70f
                } else if (0x533..=0x535).contains(&actor.motion.action) {
                    0x535
                } else {
                    0x800
                };
                actor.change_motion(action, Velocity::Motion, &assets.resources)?;
            }
            if taker == rice.owner {
                actor.inventory.counts[11] = actor.inventory.counts[11].wrapping_add(1);
                let available = actor
                    .inventory
                    .selected
                    .is_some_and(|i| actor.inventory.count(i) != 0);
                if available {
                    self.controls
                        .select_item(&mut actor.inventory, false, taker == self.player);
                }
                self.controls
                    .select_item(&mut actor.inventory, available, taker == self.player);
                recovered.push(taker);
            } else {
                actor.flags |= if self.map == 12 { BURNING } else { POISONED };
            }
            alive = false;
        }
        for taker in recovered {
            self.emit(taker, Event::Pickup(11), events)?;
            self.emit(taker, Event::SoundPosition { position, id: 7 }, events)?;
        }
        Ok(alive)
    }
}
