use super::{Context, ItemChoices, Melee};
use crate::{
    Result,
    math::Position,
    tenchu2::{actor::State, ai::Decision, input},
};
impl Melee {
    pub fn blade(&mut self, context: &mut Context<'_>) -> Result<Decision> {
        let mut decision = Decision::default();
        decision.held = self.blade_input(context, &mut decision)?;
        Ok(decision)
    }

    fn blade_input(&mut self, c: &mut Context<'_>, decision: &mut Decision) -> Result<u16> {
        if c.steering.forced || c.actor.state == State::Item {
            return Ok(0);
        }
        if c.actor.state == State::Attack {
            return c.combo(3000, 800);
        }
        if c.actor.state == State::Jumping {
            return Ok(if c.sight.distance < 3000 {
                input::ATTACK
            } else {
                0
            });
        }
        if matches!(c.actor.motion.action, 0x113 | 0x114) && c.facing < 200 {
            c.face_target();
        }
        if c.actor.motion.action == 0x916 {
            return Ok(if c.likely()? { input::DOWN } else { 0 });
        }
        if c.actor.motion.action == 0x111 && c.sight.distance > 3500 {
            return Ok(input::JUMP);
        }
        if c.actor.flags & 0x80000 != 0 {
            self.attack_delay = self.attack_delay.wrapping_add(10);
        }
        if !self.engaged {
            *decision = c.approach(&mut self.offset, 3000)?;
            if decision.held == 0 || c.observed_flags & 0x80000 != 0 {
                self.engaged = true;
            }
            self.attack_delay = 0;
            let mut held = decision.held;
            if c.sight.distance >= 8001 && c.facing < 100 {
                if c.random.next15() % 20 == 0 {
                    held = input::UP | input::JUMP;
                } else if c.random.next15() % 10 == 0
                    && c.steering.ahead.is_some_and(|p| p.height < 3000)
                {
                    held = c.command(0x111);
                }
            }
            return Ok(held);
        }
        let mut held = c.turn();
        if c.actor.motion.frame % 10 != 0 {
            return c.between_decisions(c.actor.pad.held & !input::ITEM, 2000, false);
        }
        if c.sight.distance > 5000 {
            if c.actor.inventory.counts[1] != 0 && c.low_health() {
                held = c.select_item(1)?;
            } else if c.facing < 500 && c.difficulty != 3 {
                held = c.choose_item(ItemChoices::OFFENSIVE)?;
            }
            self.engaged = false;
            self.offset = Position::default();
            return Ok(held);
        }
        if c.observed_flags & 0x8000 != 0 {
            self.engaged = false;
        }
        if c.sight.distance < 2000 {
            self.close(c, decision, held)
        } else if c.sight.distance < 3000 {
            self.middle(c, held)
        } else {
            self.far(c, decision, held)
        }
    }

    pub(super) fn close(
        &self,
        c: &mut Context<'_>,
        decision: &mut Decision,
        held: u16,
    ) -> Result<u16> {
        let bearing = i32::from(c.sight.player_angle).abs();
        if c.facing < 200 {
            if (usize::from(self.attack_delay) > c.difficulty * 10
                || (c.difficulty < 2 && bearing > 512))
                && c.likely()?
            {
                return Ok(c.attack());
            }
            return c.defend(held);
        }
        if c.facing < 512 {
            return c.defend(held);
        }
        if c.facing >= 1280 {
            if c.actor.kind == 0xee || (c.actor.kind == 0xe2 && c.sight.distance < 1000) {
                return Ok(input::ATTACK | input::UP | input::DOWN);
            }
            return Ok(if bearing < 512 {
                input::UP
            } else {
                c.turn_around()
            });
        }
        if c.low_health() && c.likely()? {
            let held = c.choose_item(ItemChoices::DEFENSIVE)?;
            if held != 0 {
                return Ok(held);
            }
            if c.actor.kind == 0xe0 && c.random.next15() & 3 == 0 {
                decision.action = Some(0x70d);
                return Ok(0);
            }
            return Ok(input::ATTACK);
        }
        if c.target.state == State::Attack && bearing < 512 {
            return c.guard(held);
        }
        if usize::from(self.attack_delay) <= c.difficulty * 10 || !c.likely()? {
            return Ok(held);
        }
        Ok(if c.random.next15() & 3 == 0 {
            input::ATTACK | input::UP | input::DOWN
        } else {
            held | input::ATTACK
        })
    }

    fn middle(&self, c: &mut Context<'_>, mut held: u16) -> Result<u16> {
        if c.random.next15() & 15 == 0 && c.facing < 500 {
            held = c.choose_item(ItemChoices::DEFENSIVE)?;
            if held & input::ITEM != 0 {
                return Ok(held);
            }
        }
        if c.facing >= 512 {
            return Ok(c.turn_around());
        }
        if c.low_health() && c.likely()? {
            held = c.choose_item(ItemChoices::BOTH)?;
            if held == 0 || c.target.state == State::Hurt {
                held = input::ATTACK;
            }
            if c.actor.kind == 0xf1 {
                held = input::ATTACK | input::UP | input::DOWN;
            }
            return Ok(held);
        }
        if c.target.state == State::Attack && i32::from(c.sight.player_angle).abs() < 512 {
            return c.guard(held);
        }
        Ok(if c.random.next15() & 3 == 0 {
            held
        } else {
            input::UP
        })
    }

    fn far(&self, c: &mut Context<'_>, decision: &mut Decision, mut held: u16) -> Result<u16> {
        if c.actor.kind == 0xe0 && c.facing < 512 && c.random.next15() & 3 == 0 {
            decision.action = Some(0x70d);
            return Ok(held);
        }
        if c.actor.kind == 0xf1 && c.facing < 512 && c.random.next15() & 3 == 0 {
            return Ok(input::ATTACK | input::UP | input::DOWN);
        }
        if c.sight.distance < 3901 || c.facing >= 500 {
            if c.random.next15() & 1 != 0 && c.facing < 500 {
                held = c.choose_item(ItemChoices::DEFENSIVE)?;
            }
            if held & input::ITEM == 0 {
                held |= if c.random.next15() % (c.difficulty as i32 + 1) == 0
                    && c.player_held & input::ATTACK != 0
                    && c.facing < 800
                {
                    input::DOWN
                } else {
                    input::UP
                };
            }
            return Ok(held);
        }
        c.maneuver(decision, held)
    }
}
