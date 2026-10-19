use super::*;

impl Controller<'_, '_> {
    pub(super) fn contact(&mut self) -> Result<u16> {
        if self.attribute & ATTR_WALL != 0 && self.actor.pad.hold_counter == 0 {
            self.hold(if self.degree() > 0 { RIGHT } else { LEFT }, 8);
        }
        self.goto(0, 0)
    }

    pub(super) fn abandon(&mut self) -> Result<u16> {
        let base = self.attribute & !(ATTR_SEARCH | ATTR_PHASE);
        self.actor.brain.chase = [0; 2];
        if self.actor.kind & 0xf0 == 0x80 {
            if matches!(self.state.detection, Detection::Seen | Detection::Glimpse) {
                self.attribute = base | 2;
                if self.actor.motion.count == 0 && self.random() % 60 == 0 {
                    self.voice(13);
                }
            }
            return Ok(self.goto(0, 0)? & (LEFT | RIGHT));
        }
        if self.state.alert != 0 {
            if self.state.detection == Detection::Seen {
                self.attribute = base | 2;
            }
            return Ok(self.goto(0, 0)? & (LEFT | RIGHT));
        }
        if self.actor.think[3] == 3 {
            let pad = self.goto(0, 0)? & (LEFT | RIGHT);
            if pad != 0 {
                return Ok(pad);
            }
        }
        match self.state.detection {
            Detection::Gone | Detection::Unseen => {
                self.attribute = base;
                self.motion(MOT_STATE_SHEATHE);
                self.voice(14);
            }
            Detection::Glimpse => {
                self.attribute = base | 1;
                self.motion(MOT_STATE_SHEATHE);
            }
            Detection::Seen => self.attribute = base | 2,
            Detection::None => {}
        }
        Ok(0)
    }

    pub(super) fn investigate(&mut self, chase: bool) -> Result<u16> {
        if self.state.detection == Detection::Seen {
            self.attribute = (self.attribute & !ATTR_PHASE) | 2;
            return Ok(0);
        }
        if self.actor.brain.chase == [0; 2] {
            if self.actor.brain.count >= 91 {
                return self.abandon();
            }
            self.actor.brain.count = self.actor.brain.count.wrapping_add(1);
            let mut pad = if chase { UP } else { 0 };
            if !chase || self.actor.brain.count < 30 {
                if self.degree() > self.turn() {
                    pad |= RIGHT;
                } else if self.degree() < -self.turn() {
                    pad |= LEFT;
                }
            }
            Ok(pad)
        } else {
            self.actor.brain.timer = self.actor.brain.timer.wrapping_add(1);
            let x = self.actor.brain.chase[0].wrapping_sub(self.actor.position[0]);
            let z = self.actor.brain.chase[1].wrapping_sub(self.actor.position[2]);
            let pad = self.goto(x, z)?;
            if self.length(&[x, z]) < 1000 || self.actor.brain.timer == 0 {
                self.actor.brain.chase = [0; 2];
                self.actor.brain.count = 0;
            }
            Ok(pad)
        }
    }
}
