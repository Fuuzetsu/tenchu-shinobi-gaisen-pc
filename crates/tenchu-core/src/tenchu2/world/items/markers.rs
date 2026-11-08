use super::*;

impl World {
    pub(super) fn pick_up_marker(
        &mut self,
        index: usize,
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<bool> {
        let entry = &self.items.entries[index];
        let State::Marker(marker) = &entry.state else {
            unreachable!()
        };
        let owner = marker.owner;
        let position = marker.position;
        let color = usize::from(marker.color);
        let mut picked_up = 0;
        for contact in self.collision.contacts(entry.model) {
            let slot = self
                .collision
                .slot(contact.index)
                .ok_or("Missing Tenchu 2 marker contact")?;
            if slot.flags & BODY == 0 || slot.owner != Some(owner) {
                continue;
            }
            let actor = &mut self.actors[owner].actor;
            if !matches!(
                actor.state,
                ActorState::Normal
                    | ActorState::Ready
                    | ActorState::Crouched
                    | ActorState::Wall
                    | ActorState::Ledge
                    | ActorState::Swimming
            ) || actor.motion.action == 0x513
            {
                continue;
            }
            // Retail clears the return-point slot even when an affliction prevents pickup.
            self.items.markers[color] = None;
            if actor.flags & 0xf00 != 0 {
                continue;
            }
            if !matches!(self.camera.mode, 19 | 17) && actor.flags & 0x7000 == 0 {
                actor.change_motion(0x800, Velocity::Motion, &assets.resources)?;
            }
            actor.inventory.counts[12] = actor.inventory.counts[12].wrapping_add(1);
            let available = actor
                .inventory
                .selected
                .is_some_and(|i| actor.inventory.count(i) != 0);
            if available {
                self.controls
                    .select_item(&mut actor.inventory, false, owner == self.player);
            }
            self.controls
                .select_item(&mut actor.inventory, available, owner == self.player);
            picked_up += 1;
        }
        for _ in 0..picked_up {
            self.emit(owner, Event::Pickup(12), events)?;
            self.emit(owner, Event::SoundPosition { position, id: 7 }, events)?;
        }
        Ok(picked_up != 0)
    }
}
