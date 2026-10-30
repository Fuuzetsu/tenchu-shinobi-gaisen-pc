use super::*;
use crate::math::Position;

impl Controller<'_, '_> {
    pub fn update(&mut self) -> Result<()> {
        let strain = self.state.strain;
        let distant = !self.nearby_player();
        let result = self.transition();
        if distant {
            self.state.strain = strain;
        }
        if let Some(mut caller) = self.replaced.take() {
            std::mem::swap(self.actor, &mut caller);
            let id = caller.id;
            self.roster.actors[id] = Some(caller);
        }
        result
    }

    fn transition(&mut self) -> Result<()> {
        let saved_strain = self.state.strain;
        let base = self.attribute & !(ATTR_SEARCH | ATTR_PHASE);
        let player = self.actor.root_model == self.player.root;
        if player {
            self.state.player_strain = saved_strain;
            self.state.strain = i32::MAX;
        }
        if matches!(self.actor.status, 16 | 17) {
            if !player && self.actor.health > 0 && self.state.strain > 0 {
                self.state.strain = -32768;
            }
            self.actor.pad.update(0);
            return Ok(());
        }
        if self.state.action_halt != crate::ai::Halt::Running {
            let mut pad = 0;
            if (self.actor.kind.wrapping_sub(0x10) as u16) < 0x70 {
                let target = self.target().ok_or("Halted guard has no target")?;
                let x = target.x.wrapping_sub(self.actor.position.x);
                let z = target.z.wrapping_sub(self.actor.position.z);
                let degree = i32::from(self.ctx.math.direction(x, z, self.actor.rotation[1])?);
                if degree > self.turn() {
                    pad = RIGHT;
                } else if degree < -self.turn() {
                    pad = LEFT;
                }
                if self.length(&[x, z]) < 2000 {
                    pad |= DOWN;
                }
            }
            self.actor.pad.update(pad);
            return Ok(());
        }
        if self.attribute & ATTR_CUSTOMAI == 0 {
            if player && self.state.alert != 0 {
                self.state.alert = self.state.alert.wrapping_sub(1).max(0);
            }
            let pad = self.program(0)?;
            self.actor.pad.update(pad);
            return Ok(());
        }
        let target = self.target();
        self.state.detection = self.state.sight.search(
            crate::actor::sight::Search {
                owner: self.actor,
                target,
                player: self.player,
                clock: self.clock,
                alert: self.state.alert,
                range_percent: self.state.sight_percent,
                all_active: self.roster.keep_all_active,
            },
            self.ctx,
        )?;
        let player_distance = if self.player_target() {
            self.distance()
        } else {
            self.length(&std::array::from_fn::<_, 3, _>(|i| {
                self.player.position[i].wrapping_sub(self.actor.position[i])
            }))
        };
        if matches!(self.player.active_item, 11 | 18) {
            if !matches!(self.actor.kind & 0xf0, 0x80 | 0xa0)
                && (self.player.active_item != 18 || self.attribute & ATTR_PHASE != 2)
            {
                self.state.alert = 0;
                self.state.detection = Detection::Gone;
            }
        } else if self.hears_alert() {
            if self.state.alert == 1 {
                self.state.detection = Detection::Unseen;
            } else if self.attribute & ATTR_PHASE == 0 {
                self.state.detection = Detection::Glimpse;
            } else if self.state.detection == Detection::Glimpse {
                self.state.detection = Detection::Seen;
            }
        }
        let v = self
            .ctx
            .math
            .velocity(self.actor.rotation[1], self.width().wrapping_mul(2), 0);
        let Position { x, y, z } = self.actor.position;
        for i in 0..2 {
            let sign = if i == 0 { 1 } else { -1 };
            self.state.probe[i] = self.level(
                Position::new(
                    x.wrapping_add(i32::from(v[0]) * sign),
                    y.wrapping_sub(3050),
                    z.wrapping_add(i32::from(v[2]) * sign),
                ),
                26,
            )?;
            self.state.probe_attribute[i] = self.ctx.cursor.attribute;
        }
        if self.state.ignore_player
            && !player
            && !self.actor.scripted
            && (self.player_target() || self.actor.target.is_none())
        {
            if self.attribute & ATTR_PHASE != 0 {
                self.actor.clear_attack(self.ctx, 3)?;
                self.actor.attribute = base;
                let action = self.actor.return_normal(self.ctx, self.player.id)?;
                self.actor.enter(self.ctx, action, true);
            }
            self.attribute = base;
            self.state.alert = 0;
            let pad = if matches!(self.actor.think[0], 8 | 9) {
                0
            } else {
                self.program(0)?
            };
            self.attribute &= !(ATTR_PHASE | ATTR_SEARCH);
            self.state.detection = Detection::Gone;
            self.finish(pad & !(SQUARE | TRIANGLE), saved_strain);
            return Ok(());
        }
        let mut pad = match self.attribute & ATTR_PHASE {
            0 => {
                self.state.strain = self.state.strain.min(player_distance);
                let pad = self.program(0)?;
                if self.actor.kind >= 7 {
                    if self.state.detection == Detection::Seen {
                        self.motion(MOT_STATE_DRAW);
                        if !self.motion(MOT_ACTION_NOTICE) {
                            self.voice(13);
                        }
                        self.attribute = base | 2;
                        self.actor.brain.chase = [0; 2];
                        if self.actor.health > 0 {
                            self.alert();
                            self.found();
                        }
                    } else if self.state.detection == Detection::Glimpse {
                        if self.state.alert != 0 {
                            self.motion(MOT_STATE_DRAW);
                            self.actor.brain.chase = [0; 2];
                        }
                        self.attribute = base | 1;
                        self.voice(12);
                    }
                }
                pad
            }
            1 => {
                let pad = if self.hears_alert() || self.attribute & ATTR_SEARCH != 0 {
                    if self.state.strain > 0 {
                        self.state.strain = -32768;
                    }
                    if self.attribute & ATTR_SEARCH != 0 {
                        self.alarm()?
                    } else {
                        self.goto(0, 0)? & (LEFT | RIGHT)
                    }
                } else {
                    if self.state.strain > 0 || self.state.strain < player_distance.wrapping_neg() {
                        self.state.strain = player_distance.wrapping_neg();
                    }
                    self.program(1)?
                };
                if self.state.detection == Detection::Seen
                    || (self.attribute & ATTR_HIT != 0
                        && matches!(self.state.detection, Detection::Seen | Detection::Glimpse))
                {
                    self.attribute = base | 2;
                    if self.attribute & ATTR_WEAPON_DRAWN == 0 {
                        self.motion(MOT_STATE_DRAW);
                    }
                    self.actor.brain.chase = [0; 2];
                    self.voice(13);
                    if self.actor.health > 0 {
                        self.alert();
                        self.found();
                    }
                } else if (!self.hears_alert() || self.state.alert < 2)
                    && matches!(self.state.detection, Detection::Gone | Detection::Unseen)
                {
                    if self.actor.kind & 0xf0 != 0x80 {
                        self.motion(MOT_STATE_SHEATHE);
                    }
                    self.attribute = base;
                }
                pad
            }
            2 => {
                if self.attribute & ATTR_WEAPON_DRAWN != 0 {
                    if self.player_target() {
                        self.state.strain = 0;
                    } else if self.state.strain > 0 {
                        self.state.strain = -32768;
                    }
                } else {
                    self.state.strain = -1;
                }
                let mut pad = if self.attribute & ATTR_SEARCH != 0 {
                    self.program(2)?
                } else {
                    if self.attribute & ATTR_WEAPON_DRAWN == 0 {
                        self.motion(MOT_STATE_DRAW);
                    }
                    self.first_attack()?
                };
                if pad & SQUARE != 0 {
                    if self.player.motion == MOT_DAMAGE_DOWNED {
                        pad &= 0xf000;
                    } else {
                        let target = self.target().ok_or("Attacking actor has no target")?;
                        if target.y.wrapping_sub(self.actor.position.y).wrapping_abs() >= 2000
                            && self.class() != 3
                        {
                            pad &= 0xf000;
                        } else if self.random() % 4
                            >= self.state.difficulty.attack_chance_quarters()
                        {
                            pad = 0;
                        }
                    }
                }
                if self.state.detection == Detection::Gone {
                    self.attribute = base | ATTR_SEARCH | 3;
                    let target = self.target().ok_or("Searching actor has no last target")?;
                    self.actor.brain.chase = [target.x, target.z];
                    self.actor.brain.timer = 1;
                }
                if self.actor.pad.hold_counter == 0 {
                    if pad & DOWN != 0
                        && (self.state.probe_attribute[1] & 0x204 != 0
                            || self.state.probe[1] > 5000)
                    {
                        self.hold(UP, 30);
                    }
                    if pad & UP != 0
                        && (self.state.probe_attribute[0] & 0x204 != 0
                            || self.state.probe[0] > 5000)
                    {
                        pad = self.goto(0, 0)? & (LEFT | RIGHT);
                    }
                    if self.player.motion == MOT_SYURI_RECOVER
                        && (self.random() % (self.engage() + 1) == 0
                            || self.actor.kind & 0xf0 == 0x80)
                    {
                        let command = if self.random() & 1 != 0 { 3 } else { 4 };
                        pad = self.command(command);
                    }
                }
                pad
            }
            _ => {
                if self.state.strain > 0 {
                    self.state.strain = -32768;
                }
                let pad = self.program(3)?;
                if self.attribute & ATTR_WALL != 0 && self.actor.pad.hold_counter == 0 {
                    self.hold(if self.degree() > 0 { RIGHT } else { LEFT }, 8);
                }
                if self.attribute & ATTR_PHASE == 2 {
                    self.voice(13);
                    self.alert();
                    if self.attribute & ATTR_SEARCH == 0 {
                        self.found();
                    }
                }
                pad
            }
        };
        if self.actor.pad.hold_counter != 0 {
            pad = (self.actor.pad.hold_counter >> 16) as u16;
            let frames = i32::from(self.actor.pad.hold_counter as u8) - 1;
            if frames != 0 {
                self.hold(pad, frames);
            } else if pad & (LEFT | RIGHT) != 0 {
                let frames = (self.random() % 3 + 1) * 30;
                self.hold(UP, frames);
            } else {
                self.actor.pad.hold_counter = 0;
            }
        }
        if self.actor.status == 10 {
            pad = UP;
            if self.degree().abs() >= 500 {
                pad = DOWN;
                if self.attribute & ATTR_PHASE == 0 {
                    pad = UP;
                } else {
                    self.hold(if self.degree() > 0 { RIGHT } else { LEFT }, 15);
                }
            }
        } else if self.state.probe_attribute[0] & 0x204 != 0 && pad & UP != 0 {
            pad &= !UP;
        } else if self.state.probe_attribute[1] & 0x204 != 0 && pad & DOWN != 0 {
            pad &= !DOWN;
        } else {
            if self.actor.motion.count == 0
                && self.degree().abs() < 500
                && (self.actor.think[0] == 6
                    || (self.actor.kind & 0xf0 == 0x20
                        && self.state.difficulty != Difficulty::Easy))
            {
                if self.ninja_step()? {
                    pad = UP | CROSS;
                }
                self.finish(pad, saved_strain);
                return Ok(());
            }
            if self.clock % 90 == 0
                && (self.actor.map.attribute & 0x100 != 0
                    || (pad & UP != 0
                        && self.state.probe[0] <= 2200
                        && self.state.probe[0] != terrain::NONE)
                    || (pad & DOWN != 0
                        && self.state.probe[1] <= 2200
                        && self.state.probe[1] != terrain::NONE))
            {
                pad |= CROSS;
            }
        }
        self.finish(pad, saved_strain);
        Ok(())
    }

    fn finish(&mut self, pad: u16, saved_strain: i32) {
        if self.actor.health < 0 {
            self.state.strain = saved_strain;
        }
        self.actor.attribute = self.attribute;
        self.actor.pad.update(pad);
    }

    pub(super) fn first_attack(&mut self) -> Result<u16> {
        let mut pad = self.goto(0, 0)?;
        if self.distance() < 10000 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        if self.actor.kind & 0xf0 == 0x90 {
            self.attribute |= ATTR_SEARCH;
        }
        let class = self.class();
        if class == 3 {
            pad &= LEFT | RIGHT;
            if self.degree().abs() > 100 {
                return Ok(pad);
            }
        }
        if self.distance() < [2000, 3000, 4000, 20000][class] {
            pad |= SQUARE;
            self.attribute |= ATTR_SEARCH;
        }
        Ok(pad)
    }
}
