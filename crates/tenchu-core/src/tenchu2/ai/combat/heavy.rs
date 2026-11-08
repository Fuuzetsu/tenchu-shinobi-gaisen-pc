use super::{Context, ItemChoices, Melee};
use crate::{
    Result,
    math::Position,
    tenchu2::{actor::State, ai::Decision, input},
};

impl Melee {
    pub fn heavy(&mut self, context: &mut Context<'_>) -> Result<Decision> {
        let mut decision = Decision::default();
        decision.held = self.heavy_input(context, &mut decision)?;
        Ok(decision)
    }

    fn heavy_input(&mut self, c: &mut Context<'_>, decision: &mut Decision) -> Result<u16> {
        if c.steering.forced || c.actor.state == State::Item {
            return Ok(0);
        }
        if c.actor.state == State::Jumping {
            return Ok(if c.sight.distance < 3000 {
                input::ATTACK
            } else {
                0
            });
        }
        if c.actor.motion.action == 0x916 {
            return Ok(if c.likely()? { input::DOWN } else { 0 });
        }
        if matches!(c.actor.motion.action, 0x113 | 0x114) && c.facing < 200 {
            c.face_target();
        }
        if c.actor.flags & 0x80000 != 0 {
            self.attack_delay = self.attack_delay.wrapping_add(10);
        }
        if !self.engaged {
            *decision = c.approach(&mut self.offset, 3000)?;
            if decision.held == 0 || c.observed_flags & 0x80000 != 0 {
                self.engaged = true;
            }
            if c.sight.distance > 8000
                && c.facing < 100
                && c.actor.motion.frame == 0
                && c.random.next15() % 5 == 0
            {
                decision.held = input::UP | input::JUMP;
            }
            self.attack_delay = 0;
            return Ok(decision.held);
        }
        let mut held = c.turn();
        if c.actor.motion.frame % 10 != 0 {
            return c.between_decisions(
                c.actor.pad.held & !(input::ITEM | input::ATTACK),
                3000,
                false,
            );
        }
        if c.sight.distance > 5000 {
            if c.actor.inventory.counts[1] != 0 && c.low_health() {
                held = c.select_item(1)?;
            }
            self.engaged = false;
            self.offset = Position::default();
            return Ok(held);
        }
        if c.observed_flags & 0x8000 != 0 {
            self.engaged = false;
        }
        if c.sight.distance < 3000 {
            return c.close_combat(self.attack_delay, 200, held);
        }
        if c.sight.distance < 4000 {
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
                if held != 0 && c.target()?.state != State::Hurt {
                    return Ok(held);
                }
                return Ok(if c.actor.kind == 0xe1 && c.random.next15() & 1 != 0 {
                    input::ATTACK | input::UP | input::DOWN
                } else {
                    input::ATTACK
                });
            }
            return Ok(if c.random.next15() & 3 == 0 {
                held
            } else {
                input::UP
            });
        }
        if c.facing >= 500 {
            return Ok(if c.random.next15() & 1 != 0 {
                held | input::UP
            } else {
                held
            });
        }
        if c.random.next15() & 1 == 0 {
            if c.random.next15() & 1 != 0 {
                held = c.command(0x910);
            } else if c.actor.inventory.counts[1] != 0 && c.low_health() {
                held = c.select_item(1)?;
            } else if c.random.next15() & 3 == 0 {
                decision.action = Some(0x70d);
            }
        } else if c.steering.ahead.is_some_and(|p| p.height < 3000) {
            held = c.command(0x111);
        }
        Ok(held)
    }
}
