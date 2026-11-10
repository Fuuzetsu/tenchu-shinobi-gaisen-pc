use super::{Context, Debris, Environment, Event, Events, GAISEN_OPEN_PITFALL, GAISEN_PITFALL};
use crate::{
    Result,
    math::Position,
    tenchu2::{
        collision::{BODY, WEAPON},
        terrain::{self, Query},
    },
};
use tenchu_assets::tenchu2::level::objects::Bounds;

impl Environment {
    pub fn update_contacts(
        &mut self,
        bounds: &[Bounds],
        player: (usize, Position),
        mut enabled: impl FnMut(&super::Object) -> bool,
        c: &mut Context<'_, impl Events>,
    ) -> Result<()> {
        for index in 0..self.objects.len() {
            let object = &self.objects[index];
            if object.model.is_none() || !enabled(object) {
                continue;
            }
            let contacts: Vec<_> = c.collision.contacts(object.coordinate).collect();
            for contact in contacts {
                let slot = c
                    .collision
                    .slot(contact.index)
                    .ok_or("Missing Tenchu 2 object contact")?;
                let flags = slot.flags;
                let owner = slot.owner;
                let object = &mut self.objects[index];
                match object.kind {
                    GAISEN_PITFALL if flags & BODY != 0 => {
                        let trigger = self
                            .triggers
                            .get_mut(object.link as usize)
                            .ok_or("Missing Gaisen pitfall trigger")?;
                        for id in [trigger.target & 255, trigger.target >> 8] {
                            let leaf = self
                                .objects
                                .get_mut(usize::from(id))
                                .ok_or("Missing Gaisen pitfall leaf")?;
                            if leaf.kind == GAISEN_PITFALL {
                                c.collision.remove(leaf.coordinate);
                                leaf.kind = GAISEN_OPEN_PITFALL;
                            }
                        }
                        trigger.clock = 0x8000;
                        c.sound(trigger.position, 0x28)?;
                    }
                    0 if flags & BODY != 0 => {
                        c.collision.remove(object.coordinate);
                        object.kind = -1;
                        let trigger = self
                            .triggers
                            .get_mut(object.link as usize)
                            .ok_or("Missing Tenchu 2 door trigger")?;
                        if trigger.clock & 0x8000 == 0 {
                            trigger.clock = 0x8000;
                        }
                        let b = bounds
                            .get(object.model.ok_or("Missing Tenchu 2 door model")?)
                            .ok_or("Missing Tenchu 2 door bounds")?;
                        let stride = (i32::from(b.half_width.max(b.half_depth)) * 2) / 25;
                        let [sin, cos] = c.math.pair(object.yaw.wrapping_add(1024));
                        object.link = (((-sin >> 7) * stride) >> 5) as i16;
                        object.clock = (((-cos >> 7) * stride) >> 5) as i16;
                    }
                    1 | 2 if flags & BODY != 0 => {
                        let link = object.link as usize;
                        let trigger = self
                            .triggers
                            .get_mut(link)
                            .ok_or("Missing Tenchu 2 paired door trigger")?;
                        for (id, side) in [(trigger.target & 255, 1), (trigger.target >> 8, -1)] {
                            let door = self
                                .objects
                                .get_mut(usize::from(id))
                                .ok_or("Missing Tenchu 2 paired door")?;
                            if matches!(door.kind, 1 | 2) {
                                c.collision.remove(door.coordinate);
                                door.kind = -1;
                                let distance = if door.yaw == 0 {
                                    contact.distance.z
                                } else {
                                    contact.distance.x
                                };
                                door.clock = if distance > 0 { side * 32 } else { -side * 32 };
                            }
                        }
                        if trigger.clock & 0x8000 == 0 {
                            trigger.clock = 0x8000;
                        }
                    }
                    4 if flags & BODY != 0
                        || (flags & WEAPON != 0
                            && flags & 63 != 10
                            && c.collision.slot(index).is_none_or(|s| s.flags & 63 != 18)) =>
                    {
                        c.emit(Event::Explosion {
                            position: c.scene.models[object.coordinate].local.translation,
                            small: false,
                        })?;
                        object.model = None;
                        object.kind = -1;
                        if let Some(slot) = c.collision.get_mut(object.coordinate) {
                            slot.flags = 0x5007;
                        }
                        c.emit(Event::Alert {
                            position: object.position,
                            kind: 3,
                        })?;
                    }
                    5 if owner == Some(player.0) => {
                        if [10, 14, 18, 25, 28]
                            .into_iter()
                            .all(|mask| flags & mask != mask)
                        {
                            let trigger = self
                                .triggers
                                .get_mut(object.link as usize)
                                .ok_or("Missing Tenchu 2 hazard trigger")?;
                            if trigger.clock & 0x8000 == 0 {
                                trigger.clock = 0x8000;
                                c.rumble(owner, 1, 3)?;
                                c.sound(player.1, 0x3e)?;
                                c.emit(Event::Alert {
                                    position: player.1,
                                    kind: 3,
                                })?;
                            }
                        }
                    }
                    6 | 7 if flags & 0x100 != 0 => {
                        c.collision.remove(object.coordinate);
                        c.emit(Event::Debris(Debris {
                            model: object.model.ok_or("Missing Tenchu 2 object model")?,
                            position: c.scene.models[object.coordinate].local.translation,
                            extent: Position::default(),
                            velocity: Position::new(0, -512, 0),
                            speed: 180,
                            gravity: 10,
                            age: 0,
                        }))?;
                        if let Some(trigger) = self
                            .triggers
                            .iter_mut()
                            .find(|t| t.kind == object.kind && usize::from(t.target) == index)
                        {
                            trigger.clock = 0x8032;
                        } else {
                            object.kind = -1;
                            object.model = None;
                        }
                    }
                    10 | 11 | 12 | 13 | 15 | 16 | 20 => {
                        let kind = object.kind;
                        if kind == 20 && object.link != 0 {
                            continue;
                        }
                        let sword = kind != 10 && flags & WEAPON != 0 && flags & 63 == 0;
                        if !sword && flags & 0x100 == 0 {
                            continue;
                        }
                        if sword {
                            c.rumble(owner, 2, 1)?;
                        }
                        c.collision.remove(object.coordinate);
                        let model = object.model.ok_or("Missing Tenchu 2 breakable model")?;
                        let b = bounds
                            .get(model)
                            .ok_or("Missing Tenchu 2 breakable bounds")?;
                        let position = c.scene.models[object.coordinate].local.translation;
                        let size = Position::new(
                            i32::from(b.half_width),
                            i32::from(b.top),
                            i32::from(b.half_depth),
                        );
                        if sword || kind == 20 {
                            c.sound(object.position, 0x38)?;
                        }
                        c.emit(Event::Debris(Debris {
                            model,
                            position: if sword {
                                position.wrapping_sub(size)
                            } else {
                                position
                            },
                            extent: if sword {
                                size.map(|n| n * 2)
                            } else {
                                Position::default()
                            },
                            velocity: Position::new(0, -512, 0),
                            speed: if sword { 10 } else { 180 },
                            gravity: if sword { 7 } else { 10 },
                            age: if sword { 45 } else { 0 },
                        }))?;
                        object.model = None;
                        if kind == 11 || (kind == 15 && c.random.next15() % 3 == 0) {
                            c.emit(Event::Pickup { position, kind: 1 })?;
                        }
                        object.kind = -1;
                        if kind == 20 {
                            terrain::level(
                                c.map,
                                c.cursor,
                                object.position.wrapping_add(Position::new(0, -10, 0)),
                                Query::Uncached,
                            )?;
                            if let Some(surface) = c.cursor.last {
                                let node = &mut c.map.cells[surface.cell].nodes[surface.node];
                                if node.attributes & 0xf00 == 0x100 {
                                    node.attributes &= 0xf0ff;
                                }
                            }
                        }
                    }
                    14 if object.link == 0 && flags & WEAPON != 0 && flags & 63 == 0 => {
                        c.rumble(owner, 2, 1)?;
                        c.sound(object.position, 0x38)?;
                        c.collision.remove(object.coordinate);
                        object.actor = owner;
                        object.link = 1;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
