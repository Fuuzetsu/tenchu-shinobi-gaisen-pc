use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;

impl Controller<'_, '_> {
    pub(super) fn watch(&mut self) -> u16 {
        let mut pad = 0;
        if self.actor.brain.count & 127 == 0 {
            pad = if self.actor.brain.flag != 0 {
                RIGHT
            } else {
                LEFT
            };
            let old = self.actor.brain.timer;
            self.actor.brain.timer = old.wrapping_add(1);
            if old > 10 {
                self.actor.brain.flag = (self.random() & 1) as u8;
                self.actor.brain.timer = 0;
                self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
            }
        } else {
            self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
        }
        pad
    }

    pub(super) fn trace(&mut self) -> Result<u16> {
        let mut pad = 0;
        if self.actor.brain.count == 0 {
            let old = self.actor.brain.timer;
            self.actor.brain.timer = old.wrapping_add(1);
            if old < 60 {
                if self.degree().abs() > self.turn() && self.actor.brain.timer < 30 {
                    pad = if self.degree() > self.turn() {
                        RIGHT
                    } else {
                        LEFT
                    };
                }
            } else {
                self.actor.brain.count = 1;
                self.actor.brain.timer = 0;
            }
        } else {
            self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
            if self.attribute & ATTR_TRACE != 0 {
                pad = self.actor.patrol_input(self.ctx.math, self.turn() as i16)?;
            }
        }
        Ok(pad)
    }

    fn random_destination(&mut self) {
        self.actor.brain.chase[0] = self.actor.home[0].wrapping_add(self.random() % 10000 - 5000);
        self.actor.brain.chase[1] = self.actor.home[1].wrapping_add(self.random() % 10000 - 5000);
    }

    pub(super) fn wander(&mut self) -> Result<u16> {
        self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
        if self.actor.brain.count == 1 {
            self.random_destination();
        } else {
            let [x, z] = self.actor.brain.chase;
            let x = x.wrapping_sub(self.actor.position.x);
            let z = z.wrapping_sub(self.actor.position.z);
            if (x.wrapping_abs() < 1000 && z.wrapping_abs() < 1000)
                || self.attribute & ATTR_WALL != 0
            {
                self.actor.brain.count = 0;
            } else {
                return self.goto(x, z);
            }
        }
        Ok(0)
    }

    pub(super) fn ninja_step(&mut self) -> Result<bool> {
        let v = self
            .ctx
            .math
            .velocity(self.actor.rotation[1], self.width().wrapping_mul(5), 0);
        let Position { x, y, z } = self.actor.position;
        let y = y.wrapping_sub(3050);
        let current = self.level(Position::new(x, y, z), 25)?;
        let forward = self.level(
            Position::new(
                x.wrapping_add(i32::from(v[0])),
                y,
                z.wrapping_add(i32::from(v[2])),
            ),
            26,
        )?;
        Ok((current == self.actor.map.level && forward.wrapping_abs() < 500) || forward > 6100)
    }

    pub(super) fn ninja(&mut self) -> Result<u16> {
        if self.actor.status == ActorState::Jump {
            return Ok(0);
        }
        let old = self.actor.brain.timer;
        self.actor.brain.timer = old.wrapping_add(1);
        if old <= 30 {
            return Ok(0);
        }
        let mut pad = self.wander()?;
        if self.actor.motion.action == MOT_MOVE
            && self.actor.motion.count == 0
            && self.ninja_step()?
        {
            pad = UP | CROSS;
        }
        Ok(pad)
    }

    pub(super) fn sleep(&mut self) -> Result<u16> {
        let mut pad = 0;
        if self.actor.motion.action == MOT_ACTION {
            self.state.detection = Detection::Unseen;
        } else if self.actor.motion.count == 0 {
            pad = UP | L2;
        }
        if self.state.alert != 0 || self.attribute & ATTR_PUSH != 0 {
            pad = self.goto(0, 0)? & (LEFT | RIGHT);
        }
        Ok(pad)
    }

    pub(super) fn hunt(&mut self) -> Result<u16> {
        self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
        if self.actor.brain.count == 1 {
            if let Some(id) = self.roster.nearest(self.actor, 5000, self.ctx.math) {
                let p = self.roster.get(id).position;
                self.actor.brain.chase = [p[0], p[2]];
            } else {
                self.random_destination();
            }
            Ok(0)
        } else {
            let [x, z] = self.actor.brain.chase;
            let mut pad = self.goto(
                x.wrapping_sub(self.actor.position.x),
                z.wrapping_sub(self.actor.position.z),
            )?;
            if pad == 0 {
                pad |= SQUARE;
                self.actor.brain.count = 0;
            }
            Ok(pad)
        }
    }

    pub(super) fn follow(&mut self) -> Result<u16> {
        if self.target().is_none() {
            return Ok(self.watch());
        }
        self.state.detection = Detection::Unseen;
        if self.clock & 31 == 0 {
            let delta: Position = self.player.position.wrapping_sub(self.actor.position);
            let distance = self.length(&[delta.x, delta.z]);
            let degree = self
                .ctx
                .math
                .direction(delta.x, delta.z, self.actor.rotation[1])?;
            if distance <= 4000
                && delta.y.wrapping_abs() <= 3000
                && i32::from(degree).abs() < 900
                && self.player.active_item != 11
            {
                self.actor.target = Some(self.player.root);
                self.attribute = (self.attribute & !ATTR_PHASE) | 2;
                self.motion(MOT_STATE_DRAW);
                self.actor.brain.chase = [0; 2];
                self.voice(13);
                self.alert();
            }
        }
        let target = self.target().unwrap();
        let x = target.x.wrapping_sub(self.actor.position.x);
        let z = target.z.wrapping_sub(self.actor.position.z);
        let distance = self.length(&[x, z]);
        if distance < 200 {
            return Ok(0);
        }
        if distance < 4000 && target.y.wrapping_sub(self.actor.position.y).wrapping_abs() > 2000 {
            return Ok(self.watch());
        }
        self.goto(x, z)
    }
}
