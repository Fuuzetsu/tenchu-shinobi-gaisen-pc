use super::*;
use crate::actor::State as ActorState;

impl Controller<'_, '_> {
    pub(super) fn succession(&mut self, distance: i32, degree: i32) -> u16 {
        if self.actor.motion.count != self.ctx.mission.weapons.attacks[self.actor.attack].combo {
            return 0;
        }
        if (self.distance() < distance && self.degree().abs() < degree)
            || self.random() % (self.engage() + 1) == 0
        {
            SQUARE
                | if self.degree() > 300 {
                    RIGHT
                } else if self.degree() < -300 {
                    LEFT
                } else {
                    0
                }
        } else {
            0
        }
    }

    pub(super) fn item(&mut self) -> Result<()> {
        if self.actor.motion.count != 0 || self.actor.status != ActorState::Engage {
            return Ok(());
        }
        if self.actor.inventory[3] != 0 && self.actor.health < self.actor.max_health / 3 {
            return self.items.request_default(
                3,
                self.actor,
                self.player.id,
                self.roster,
                self.ctx,
            );
        }
        if self.actor.inventory[1] != 0 && self.degree().abs() < 100 {
            self.motion(MOT_SYURI);
        } else if self.actor.inventory[4] != 0 && self.degree().abs() < 300 {
            self.motion(MOT_ITEM_THROW);
        }
        Ok(())
    }

    pub(super) fn attack_chase(&mut self) -> Result<u16> {
        if self.distance() < 10000 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        if self.state.attack_clock.wrapping_add(self.engage() * 30) < self.clock
            && self.degree().abs() < 500
        {
            let pad = if self.distance() > 4000 {
                self.actor.pad.held
            } else if self.distance() > 3000 {
                self.command(0x21)
            } else if self.distance() < 2000 {
                SQUARE | CIRCLE
            } else {
                SQUARE
            };
            self.state.attack_clock = self.clock.wrapping_add(self.engage() * 10);
            Ok(pad)
        } else {
            self.attack_class()
        }
    }

    pub(super) fn attack_point(&mut self) -> Result<u16> {
        let class = self.class();
        if self.actor.status == ActorState::Attack {
            return Ok(if class == 3 {
                self.succession(20000, 500)
            } else {
                self.succession(3000, 1500)
            });
        }
        if self.state.detection != Detection::Gone
            && ((class == 3 && self.distance() < 14000) || self.distance() < 10000)
        {
            self.state.detection = Detection::None;
        }
        let turn = i32::from(((4 - class as i32) * self.turn()) as i16);
        let mut pad = if self.degree() > turn {
            RIGHT
        } else if self.degree() < -turn {
            LEFT
        } else {
            0
        };
        let range = [3000, 3500, 4000, 20000][class];
        let close = if class == 3 { 4000 } else { range / 2 };
        if self.distance() < close {
            if self.degree().abs() < 1000 && self.actor.motion.count == 0 {
                if self.distance() < 2000 {
                    if self.random() % (self.engage() + 1) != 0 {
                        pad |= SQUARE;
                    } else {
                        pad = SQUARE | CIRCLE;
                    }
                } else {
                    pad |= SQUARE;
                }
            } else {
                pad |= DOWN;
            }
        } else if self.distance() < range {
            if class == 3 {
                if pad == 0 && self.random() % (self.engage() * 4) == 0 {
                    pad = SQUARE;
                }
            } else if self.degree().abs() < 100 && range - 1000 < self.distance() {
                pad = self.command(0x21);
            } else if self.actor.motion.count == 0 && self.degree().abs() < 1200 {
                pad |= SQUARE;
            } else if close + 500 < self.distance() {
                pad |= UP;
            }
        } else if self.actor.status == ActorState::Engage {
            if self.player.status == ActorState::AimShuriken {
                let command = if self.random() & 1 != 0 { 3 } else { 4 };
                pad = self.command(command);
            } else if self.actor.motion.count == 0 && self.random() % 3 == 0 {
                pad = self.command(1);
            } else {
                self.item()?;
            }
        }
        if self.actor.motion.count == 0
            && self.random() % 30 == 0
            && self.actor.status == ActorState::Engage
        {
            self.motion(MOT_ATTACK_TAUNT);
        }
        Ok(pad)
    }

    pub(super) fn escape(&mut self) -> Result<u16> {
        if self.distance() < 16500 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        let degree = self.degree();
        let mut pad = if degree > 0 {
            LEFT
        } else if degree < 0 {
            RIGHT
        } else {
            0
        };
        pad |= if degree.abs() < 1000 { DOWN } else { UP };
        if self.attribute & ATTR_WALL != 0
            && degree.abs() > 1000
            && self.actor.pad.hold_counter == 0
        {
            let ticks = 1000i32
                .checked_div(self.turn())
                .ok_or("Zero AI turn speed")?;
            self.hold(if degree > 0 { RIGHT } else { LEFT }, ticks);
        }
        Ok(pad)
    }

    pub(super) fn attack_area(&mut self) -> Result<u16> {
        if self.actor.status == ActorState::Attack {
            return Ok(self.succession(4000, 500));
        }
        if self.distance() < 10000 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        if self.class() == 3 {
            return self.attack_point();
        }
        let x = self.actor.home[0].wrapping_sub(self.actor.position.x);
        let z = self.actor.home[1].wrapping_sub(self.actor.position.z);
        let distance = self.length(&[x, z]);
        let mut pad = 0;
        if self.actor.brain.flag != 0 {
            pad = self.attack_class()?;
            if self.distance() < 4000 {
                self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
                if self.actor.brain.count == 0 && distance > 4000 {
                    self.actor.brain.flag = 0;
                }
            }
        } else {
            if self.attribute & ATTR_HIT != 0 {
                self.actor.brain.flag = 1;
            }
            if distance < 2000 {
                if self.actor.motion.count & 7 != 0 {
                    pad = self.actor.pad.held;
                } else if self.degree() > 500 {
                    pad = RIGHT;
                } else if self.degree() < -500 {
                    pad = LEFT;
                } else if self.random() % 10 == 0 {
                    self.motion(MOT_ATTACK_TAUNT);
                    self.actor.brain.flag = 1;
                }
                if self.distance() < 4000 {
                    if self.degree().abs() < 100 {
                        pad = self.command(0x21);
                    } else if self.degree().abs() < 1000 {
                        pad |= SQUARE;
                    }
                    self.actor.brain.flag = 1;
                }
            } else {
                pad = self.goto(x, z)?;
                if self.attribute & ATTR_WALL != 0 {
                    self.actor.brain.flag = 1;
                }
                if self.actor.motion.count == 0
                    && self.distance() < 3000
                    && self.degree().abs() < 1000
                {
                    pad |= SQUARE;
                }
            }
        }
        Ok(pad)
    }

    pub(super) fn hit_away(&mut self) -> Result<u16> {
        if self.distance() < 10000 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        if self.actor.status == ActorState::Attack {
            self.actor.brain.flag = 0;
            self.actor.brain.chase = [0; 2];
            return Ok(self.succession(3000, 1500));
        }
        if self.actor.brain.flag != 0 {
            return self.attack_class();
        }
        let mut pad = if self.degree().abs() < 1000 {
            (self.goto(0, 0)? & (LEFT | DOWN | RIGHT)) | DOWN
        } else {
            self.chase(5000)?
        };
        if self.distance() < 2000 && self.random() % 30 == 0 {
            pad |= CROSS;
        }
        if self.distance() > 4000 || self.attribute & ATTR_WALL != 0 {
            self.actor.brain.flag = 1;
        }
        Ok(pad)
    }

    pub(super) fn attack_class(&mut self) -> Result<u16> {
        match self.class() {
            0 if self.actor.kind & 0xf0 == 0xa0 => self.animal(),
            0 => self.short(),
            1 => self.general(),
            2 => self.long(),
            3 => self.ranged(),
            _ => Err("Invalid AI weapon class".into()),
        }
    }

    pub(super) fn animal(&mut self) -> Result<u16> {
        if matches!(self.actor.status, ActorState::Attack | ActorState::Jump) {
            self.actor.brain.mode = 0;
            return Ok(0);
        }
        if self.distance() < 2000 && self.degree().abs() < 200 {
            return Ok(SQUARE);
        }
        self.actor.brain.mode = self.actor.brain.mode.wrapping_add(1);
        let mut pad = self.goto(0, 0)?;
        let timer = self.actor.brain.mode;
        if timer < 30 {
            pad = UP;
        } else if timer == 30 {
            self.voice(12);
        } else if timer < 90 {
            pad &= LEFT | RIGHT;
        }
        Ok(pad)
    }

    pub(super) fn ranged(&mut self) -> Result<u16> {
        if self.actor.status == ActorState::Attack {
            return Ok(self.succession(20000, 500));
        }
        if self.actor.status == ActorState::Jump {
            return Ok(0);
        }
        if self.distance() < 20000 && self.state.detection != Detection::Gone {
            self.state.detection = Detection::None;
        }
        let mut pad = 0;
        if self.distance() < 5000 {
            if self.degree().abs() < 1000 {
                pad = self.goto(0, 0)? & (LEFT | RIGHT);
                if self.distance() < 1000 || self.distance() > 3000 {
                    pad |= DOWN;
                }
                if self.degree().abs() < 200 && self.actor.motion.action == MOT_ENGAGE_STANCE {
                    pad = SQUARE;
                }
            } else {
                pad = UP;
            }
        } else {
            if self.random() % (self.engage() * 4) == 0
                && self.degree().abs() < 200
                && self.actor.motion.action == MOT_ENGAGE_STANCE
            {
                pad = SQUARE;
            }
            if self.distance() > 15000 {
                pad = self.goto(0, 0)?;
            } else if self.degree() > 200 {
                pad = RIGHT;
            } else if self.degree() > 100 {
                pad = self.command(4);
            } else if self.degree() < -200 {
                pad = LEFT;
            } else if self.degree() < -100 {
                pad = self.command(3);
            } else {
                self.item()?;
            }
        }
        if pad == SQUARE {
            self.actor.rotation[1] =
                self.actor.rotation[1].wrapping_add(self.state.sight.direction);
        }
        Ok(pad)
    }
}
