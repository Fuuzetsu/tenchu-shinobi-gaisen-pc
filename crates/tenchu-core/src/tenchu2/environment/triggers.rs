use super::{
    Context, Debris, Environment, Event, Events, GAISEN_PITFALL, Shot, SurfaceActor, SurfaceContext,
};
use crate::{
    Result,
    math::Position,
    tenchu2::terrain::{self, Probe, Query},
};

impl Environment {
    pub fn surface(
        &mut self,
        actor: SurfaceActor,
        probe: &Probe,
        hidden: bool,
        c: &mut SurfaceContext<'_, impl Events>,
    ) -> Result<i16> {
        if hidden || probe.material & 0xf00 != 0x400 {
            return Ok(-1);
        }
        let trigger_count = self.triggers.len() as u16;
        let Some(trigger) = self.triggers.get_mut(usize::from(probe.material & 255)) else {
            return Ok(-1);
        };
        let kind = trigger.kind;
        if kind == -1 {
            return Ok(-1);
        }
        if kind == 19 {
            if actor.focused && actor.health < 1 {
                self.background_only = true;
            }
            return Ok(kind);
        }
        if actor.health < 1 || (!actor.focused && !actor.visible) {
            return Ok(-1);
        }
        let floor = probe.floor.unwrap_or(i32::MIN);
        if probe.material & 0xc000 == 0 {
            if actor.position.y != floor {
                return Ok(-1);
            }
        } else {
            let end = floor.wrapping_add(probe.height);
            if actor.position.y < floor.min(end) || actor.position.y > floor.max(end) {
                return Ok(-1);
            }
        }
        let active = trigger.clock & 0x8000 != 0;
        match kind {
            0..=3 | 9 => {
                let object = self
                    .objects
                    .get_mut(usize::from(trigger.target))
                    .ok_or("Missing Tenchu 2 trigger object")?;
                if active || object.model.is_none() {
                    return Ok(-1);
                }
                trigger.clock = 0x8000;
                if kind == 9 {
                    terrain::level(c.map, c.cursor, trigger.position, Query::Uncached)?;
                    if let Some(surface) = c.cursor.current {
                        c.map.cells[surface.cell].nodes[surface.node].attributes =
                            0x400 | (trigger_count & 255);
                    }
                    let [sin, cos] = c.math.pair(object.yaw);
                    trigger.position.x = (sin >> 12) * -200;
                    trigger.position.z = (cos >> 12) * -200;
                }
            }
            6 | 7 => {
                let object = self
                    .objects
                    .get_mut(usize::from(trigger.target))
                    .ok_or("Missing Tenchu 2 trigger object")?;
                if !active {
                    trigger.clock |= 0x8000;
                    object.actor = Some(actor.index);
                } else if actor.focused {
                    object.actor = Some(actor.index);
                }
            }
            10..=15 => {
                if active {
                    return Ok(-1);
                }
                trigger.clock = 0x8000;
                let position = trigger.position;
                let [sin, cos] = c.math.pair(trigger.angle.wrapping_shl(4));
                let (kind, velocity) = match kind {
                    10 | 11 => {
                        c.events
                            .emit(Event::Sound { position, id: 0x3f }, c.random)?;
                        (
                            Shot::Spear,
                            Position::new(((-sin >> 7) * 1000) >> 5, 0, ((-cos >> 7) * 1000) >> 5),
                        )
                    }
                    12 | 13 => {
                        c.events
                            .emit(Event::Sound { position, id: 0x3d }, c.random)?;
                        let mut target = actor.position;
                        target.y = target.y.wrapping_sub(i32::from(actor.height) / 2);
                        (Shot::Arrow, target.wrapping_sub(position))
                    }
                    _ => (
                        Shot::Smoke,
                        Position::new(((sin >> 7) * 5) >> 4, 0, ((cos >> 7) * 5) >> 4),
                    ),
                };
                c.events.emit(
                    Event::Shoot {
                        kind,
                        position,
                        velocity,
                    },
                    c.random,
                )?;
            }
            _ => {}
        }
        Ok(kind)
    }

    pub fn update_triggers(&mut self, c: &mut Context<'_, impl Events>) -> Result<()> {
        let count = self.triggers.len() as u16;
        for (index, trigger) in self.triggers.iter_mut().enumerate() {
            if trigger.clock & 0x8000 == 0 {
                continue;
            }
            match trigger.kind {
                GAISEN_PITFALL => {
                    trigger.clock = trigger.clock.wrapping_add(1);
                    let angle = (i32::from(trigger.clock & 0x7fff) * 170).min(1024);
                    for (id, sign) in [(trigger.target & 255, -1), (trigger.target >> 8, 1)] {
                        let leaf = self
                            .objects
                            .get_mut(usize::from(id))
                            .ok_or("Missing Gaisen pitfall leaf")?;
                        if leaf.model.is_some() {
                            leaf.roll = angle * sign;
                        }
                    }
                    if angle == 1024 {
                        trigger.clock = 0;
                    }
                }
                0 => {
                    trigger.clock = trigger.clock.wrapping_add(1);
                    if trigger.clock & 0x7fff > 25 {
                        trigger.clock = 0;
                    }
                    let object = self
                        .objects
                        .get_mut(usize::from(trigger.target))
                        .ok_or("Missing Tenchu 2 sliding door")?;
                    object.position.x = object.position.x.wrapping_add(i32::from(object.link));
                    object.position.z = object.position.z.wrapping_add(i32::from(object.clock));
                }
                1 | 2 => {
                    trigger.clock = trigger.clock.wrapping_add(1);
                    if trigger.clock & 0x7fff > 32 {
                        trigger.clock = 0;
                    }
                    for id in [trigger.target & 255, trigger.target >> 8] {
                        let object = self
                            .objects
                            .get_mut(usize::from(id))
                            .ok_or("Missing Tenchu 2 hinged door")?;
                        if object.link == index as i16 {
                            object.yaw = object.yaw.wrapping_add(object.clock) & 4095;
                        }
                    }
                    if trigger.clock & 0x7fff == 1 {
                        c.sound(
                            self.objects[usize::from(trigger.target & 255)].position,
                            0x28,
                        )?;
                    }
                }
                5 | 10..=15 => {
                    trigger.clock = trigger.clock.wrapping_add(1);
                    let limit = match trigger.kind {
                        5 => 61,
                        10..=13 => 151,
                        _ => 301,
                    };
                    if trigger.clock & 0x7fff >= limit {
                        trigger.clock = 0;
                    }
                }
                6 | 7 => {
                    trigger.clock = trigger.clock.wrapping_add(1) & 0x7fff;
                    let object = self
                        .objects
                        .get_mut(usize::from(trigger.target))
                        .ok_or("Missing Tenchu 2 collapsing platform")?;
                    c.rumble(object.actor, 2, 1)?;
                    if trigger.kind == 7 {
                        terrain::level(
                            c.map,
                            c.cursor,
                            object.position.wrapping_add(Position::new(0, -500, 0)),
                            Query::Uncached,
                        )?;
                        if let Some(surface) = c.cursor.current {
                            let node = &mut c.map.cells[surface.cell].nodes[surface.node];
                            node.y = node.y.wrapping_add(2);
                        }
                        object.position.y = object.position.y.wrapping_add(20);
                    }
                    c.sound(object.position, 0x3c)?;
                    let limit = if trigger.kind == 6 { 30 } else { 40 };
                    if trigger.clock > limit {
                        let position = c.scene.models[object.coordinate]
                            .local
                            .translation
                            .wrapping_add(Position::new(-1000, 1000, -1000));
                        if trigger.kind == 7 && trigger.clock == 41 {
                            c.sound(object.position, 0x38)?;
                        }
                        if trigger.clock == limit + 1 {
                            c.emit(Event::Debris(Debris {
                                model: object.model.ok_or("Missing Tenchu 2 falling model")?,
                                position,
                                extent: Position::new(2000, 100, 2000),
                                velocity: Position::new(0, -128, 0),
                                speed: 10,
                                gravity: 15,
                                age: 0,
                            }))?;
                        }
                        if trigger.kind == 6 {
                            c.sound(object.position, 0x38)?;
                            terrain::level(
                                c.map,
                                c.cursor,
                                object.position.wrapping_add(Position::new(0, -500, 0)),
                                Query::Uncached,
                            )?;
                        }
                        if let Some(surface) = c.cursor.current {
                            let node = &mut c.map.cells[surface.cell].nodes[surface.node];
                            if trigger.kind == 7 {
                                node.y = 0;
                            }
                            node.division = 0;
                            node.attributes = 0x4400 | (count & 255);
                        }
                        trigger.clock = 0;
                        object.model = None;
                        object.kind = -1;
                    }
                }
                9 => {
                    let object = self
                        .objects
                        .get_mut(usize::from(trigger.target))
                        .ok_or("Missing Tenchu 2 rolling rock")?;
                    let position = Position::new(
                        object.position.x.wrapping_add(trigger.position.x),
                        object.position.y,
                        object.position.z.wrapping_add(trigger.position.z),
                    );
                    let floor = terrain::level(c.map, c.cursor, position, Query::Uncached)?;
                    if floor.is_none_or(|y| y.wrapping_sub(250) < object.position.y) {
                        trigger.clock = 0;
                        if let Some(slot) = c.collision.get_mut(object.coordinate) {
                            slot.flags = 0x1000;
                        }
                        c.sound(object.position, 0x33)?;
                        c.emit(Event::Debris(Debris {
                            model: object.model.ok_or("Missing Tenchu 2 rock model")?,
                            position: c.scene.models[object.coordinate].local.translation,
                            extent: Position::new(0, -1500, 0),
                            velocity: Position::new(0, -512, 0),
                            speed: 180,
                            gravity: 10,
                            age: 0,
                        }))?;
                        object.model = None;
                        object.kind = -1;
                        c.emit(Event::Alert {
                            position: object.position,
                            kind: 3,
                        })?;
                        continue;
                    }
                    if let Some(slot) = c.collision.get_mut(object.coordinate) {
                        slot.flags = 0x501d;
                    }
                    let mut floor = floor.unwrap();
                    let mut dx = trigger.position.x;
                    let mut dz = trigger.position.z;
                    let y = object.position.y;
                    object.position.y = floor.wrapping_sub(500);
                    if floor.wrapping_sub(y.wrapping_add(500)).wrapping_abs() > 150 {
                        floor = y.wrapping_add(750);
                        dx >>= 1;
                        dz >>= 1;
                        object.position.y = y.wrapping_add(250);
                    }
                    object.position.x = object.position.x.wrapping_add(dx);
                    object.position.z = object.position.z.wrapping_add(dz);
                    object.roll = object.roll.wrapping_add(113);
                    let mut transform = c.scene.models[object.coordinate].local;
                    transform.translation.x = transform.translation.x.wrapping_add(dx);
                    transform.translation.z = transform.translation.z.wrapping_add(dz);
                    transform.translation.y = floor;
                    c.scene.update(object.coordinate, transform);
                    if trigger.clock & 15 == 0 {
                        c.sound(object.position, 0x37)?;
                    }
                    trigger.clock = trigger.clock.wrapping_add(1);
                }
                _ => {}
            }
        }
        Ok(())
    }
}
