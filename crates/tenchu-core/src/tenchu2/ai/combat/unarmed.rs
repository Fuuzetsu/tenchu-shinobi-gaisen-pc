use super::{Context, Dog, ItemChoices, Melee};
use crate::{
    Result,
    tenchu2::{actor::State, ai::Decision, input},
};

impl Melee {
    pub fn unarmed(&mut self, context: &mut Context<'_>) -> Result<Decision> {
        let mut decision = Decision::default();
        decision.held = self.unarmed_input(context, &mut decision)?;
        Ok(decision)
    }

    fn unarmed_input(&mut self, c: &mut Context<'_>, decision: &mut Decision) -> Result<u16> {
        if c.steering.forced || c.actor.state == State::Item {
            return Ok(0);
        }
        if c.actor.state == State::Attack {
            return c.combo(2500, 1000);
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
            if c.sight.distance >= 2500 || c.facing >= 1000 {
                *decision = c.approach(&mut self.offset, 2000)?;
                if decision.held == 0 || c.observed_flags & 0x80000 != 0 {
                    self.engaged = true;
                }
                if c.sight.distance > 5000 && c.facing < 100 && c.random.next15() & 31 == 0 {
                    decision.held = input::UP;
                }
                self.attack_delay = 0;
                return Ok(decision.held);
            }
            if c.actor.motion.frame % 10 != 0 {
                return Ok(0);
            }
            self.engaged = true;
            return Ok(input::ATTACK
                | if c.sight.angle > 500 {
                    input::RIGHT
                } else if c.sight.angle < -500 {
                    input::LEFT
                } else {
                    0
                });
        }
        let mut held = c.turn();
        if c.actor.motion.frame % 10 != 0 {
            return c.between_decisions(c.actor.pad.held & !input::ITEM, 2000, true);
        }
        if c.sight.distance > 4000 {
            if c.actor.inventory.counts[1] != 0 && c.low_health() {
                held = c.select_item(1)?;
            } else if c.facing < 500 && c.difficulty != 3 {
                held = c.choose_item(ItemChoices::OFFENSIVE)?;
            }
            self.engaged = false;
            return Ok(if c.sight.distance > 5000 {
                input::UP
            } else {
                held
            });
        }
        if c.observed_flags & 0x8000 != 0 {
            self.engaged = false;
        }
        if c.sight.distance < 2000 {
            return c.close_combat(self.attack_delay, 300, held);
        }
        if c.sight.distance < 3000 {
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
                return Ok(if held == 0 || c.target()?.state == State::Hurt {
                    input::ATTACK
                } else {
                    held
                });
            }
            if c.target()?.state == State::Attack && i32::from(c.sight.player_angle).abs() < 512 {
                return c.guard(held);
            }
            return Ok(if c.random.next15() & 3 == 0 {
                held
            } else {
                input::UP
            });
        }
        if c.actor.kind == 0xe9 && c.random.next15() & 1 != 0 {
            return Ok(input::ATTACK
                | if c.facing >= 512 {
                    c.turn_around()
                } else {
                    input::UP | input::DOWN
                });
        }
        if c.facing < 500 && c.sight.distance > 3500 {
            return c.maneuver(decision, held);
        }
        if c.random.next15() & 1 != 0 && c.facing < 500 {
            return Ok(input::UP | input::JUMP);
        }
        if c.random.next15() & 1 != 0 {
            return Ok(held | input::UP);
        }
        if c.facing < 500 {
            if c.actor.inventory.counts[6] != 0 {
                return c.select_item(6);
            }
            if c.actor.inventory.counts[10] != 0 {
                return c.select_item(10);
            }
        }
        Ok(held)
    }
}

impl Dog {
    pub fn update(&mut self, c: &Context<'_>) -> Result<Decision> {
        if c.steering.forced {
            return Ok(Decision::default());
        }
        if matches!(c.actor.state, State::Attack | State::Jumping) {
            self.waiting = 0;
            return Ok(Decision::default());
        }
        if c.sight.distance < 2000 && c.facing < 200 {
            return Ok(Decision {
                held: input::ATTACK,
                ..Default::default()
            });
        }
        self.waiting = self.waiting.wrapping_add(1);
        let mut decision = c.steer()?;
        if self.waiting < 30 {
            decision.held = input::UP;
        } else if self.waiting < 90 {
            decision.held &= input::LEFT | input::RIGHT;
        }
        decision.held &= !(c.actor.pad.held & input::ATTACK);
        Ok(decision)
    }
}
