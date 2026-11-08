use super::{
    Decision, SteeringInput,
    sight::{Sight, Visibility},
    steer,
};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State},
        input,
        inventory::Item,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{Edition, combat::Rules};

mod blade;
pub use blade::Blade;

pub struct Context<'a> {
    pub actor: &'a mut Actor,
    pub target: &'a Actor,
    pub sight: &'a mut Sight,
    pub visibility: &'a mut Visibility,
    pub steering: SteeringInput,
    pub observed_flags: u32,
    pub player_held: u16,
    pub facing: i32,
    pub difficulty: usize,
    pub rules: &'a Rules,
    pub resources: &'a Resources,
    pub random: &'a mut Random,
}

#[derive(Clone, Copy)]
pub struct ItemChoices {
    pub defensive: bool,
    pub offensive: bool,
}

impl ItemChoices {
    const DEFENSIVE: Self = Self {
        defensive: true,
        offensive: false,
    };
    const OFFENSIVE: Self = Self {
        defensive: false,
        offensive: true,
    };
    const BOTH: Self = Self {
        defensive: true,
        offensive: true,
    };
}

impl Context<'_> {
    pub fn combo(&mut self, reach: i32, cone: i16) -> Result<u16> {
        let attack = self
            .actor
            .attack
            .index
            .and_then(|i| self.resources.weapons.attacks.get(i))
            .ok_or("Missing Tenchu 2 AI attack")?;
        if self.actor.motion.frame != i16::from(attack.combo) {
            return Ok(0);
        }
        let mut held = 0;
        if (self.sight.distance < reach && self.facing < i32::from(cone))
            || self.random.next15() % (self.difficulty as i32 + 1) == 0
        {
            held = input::ATTACK;
            if self.sight.angle >= 301 {
                held |= input::RIGHT;
            } else if self.sight.angle <= -301 {
                held |= input::LEFT;
            }
        }
        Ok(held & !(self.actor.pad.held & input::ATTACK))
    }

    fn select_item(&mut self, id: u8) -> Result<u16> {
        self.actor.inventory.selected = Some(Item::try_from(id)?);
        Ok(input::ITEM)
    }

    pub fn choose_item(&mut self, choices: ItemChoices) -> Result<u16> {
        if self.difficulty == 3 {
            return Ok(0);
        }
        let divisor = self
            .difficulty
            .checked_sub(1)
            .and_then(|index| self.rules.item_odds.get(index))
            .copied()
            .ok_or("Invalid Tenchu 2 AI difficulty")?;
        let chance = |random: &mut Random| random.next15() % i32::from(divisor) == 0;
        if choices.defensive {
            if chance(self.random)
                && self.actor.inventory.counts[6] != 0
                && self.target.flags & 0x80000 == 0
            {
                return self.select_item(6);
            }
            if chance(self.random)
                && self.actor.inventory.counts[10] != 0
                && self.target.flags & 0x80200 == 0
            {
                return self.select_item(10);
            }
        }
        if choices.offensive {
            if chance(self.random)
                && self.actor.inventory.counts[4] != 0
                && self.target.flags & 0x80000 == 0
            {
                return self.select_item(4);
            }
            if chance(self.random)
                && self.actor.inventory.counts[8] != 0
                && self.target.flags & 0x80100 == 0
            {
                return self.select_item(8);
            }
        }
        if self.resources.registry.edition == Edition::Japan
            && chance(self.random)
            && self.actor.inventory.counts[15] != 0
            && self.actor.health < self.actor.max_health / 2
        {
            return self.select_item(15);
        }
        Ok(0)
    }

    fn steer(&self) -> Result<Decision> {
        self.steer_to(0, 0)
    }

    fn steer_to(&self, x: i32, z: i32) -> Result<Decision> {
        steer(
            x,
            z,
            &SteeringInput {
                yaw: self.actor.rotation.y,
                turn: self.actor.turn,
                target_angle: self.sight.angle,
                ..self.steering
            },
            &self.resources.math,
        )
    }

    pub fn approach(&mut self, offset: &mut Position, reach: i32) -> Result<Decision> {
        if self.actor.target.is_none() {
            return Ok(Decision::default());
        }
        let x = self
            .target
            .position
            .x
            .wrapping_add(offset.x)
            .wrapping_sub(self.actor.position.x);
        let z = self
            .target
            .position
            .z
            .wrapping_add(offset.z)
            .wrapping_sub(self.actor.position.z);
        if (x.wrapping_abs() < 500 && z.wrapping_abs() < 500) || self.sight.distance < reach {
            return Ok(Decision::default());
        }
        if offset.x == 0 && offset.z == 0 {
            let angle = (self.random.next15() & 2047) as i16;
            let [s, c] = self.resources.math.pair(angle);
            let forward = (s.wrapping_mul(reach).wrapping_mul(16) >> 16) as i16;
            let side = (c.wrapping_mul(reach) >> 12) as i16;
            let [s, c] = self.resources.math.pair(self.actor.rotation.y).map(|v| -v);
            *offset = Position::new(
                s.wrapping_mul(i32::from(forward))
                    .wrapping_sub(c.wrapping_mul(i32::from(side)))
                    >> 12,
                0,
                c.wrapping_mul(i32::from(forward))
                    .wrapping_add(s.wrapping_mul(i32::from(side)))
                    >> 12,
            );
        }
        let mut decision = self.steer_to(x, z)?;
        if (self.observed_flags & 0x8000 != 0 && decision.held & (input::LEFT | input::RIGHT) != 0)
            || self.facing < 300
        {
            decision.held |= input::UP;
        }
        Ok(decision)
    }

    fn command(&mut self, action: i16) -> u16 {
        let commands = if action == 0x910 {
            &self.resources.commands.jumping
        } else {
            &self.resources.commands.standing
        };
        self.actor.pad.issue(commands, action)
    }

    fn low_health(&self) -> bool {
        i32::from(self.actor.health) < (i32::from(self.actor.max_health) >> 1)
    }

    fn likely(&mut self) -> Result<bool> {
        Ok(self.random.next15() & 255 < self.probability()?)
    }

    fn guard(&mut self, held: u16) -> Result<u16> {
        Ok(if self.likely()? {
            held | input::DOWN
        } else {
            held
        })
    }

    fn attack(&mut self) -> u16 {
        if self.random.next15() & 3 == 0 {
            input::ATTACK | input::UP | input::DOWN
        } else {
            input::ATTACK
        }
    }

    fn turn(&self) -> u16 {
        if self.sight.angle > 200 {
            input::RIGHT
        } else if self.sight.angle < -200 {
            input::LEFT
        } else {
            0
        }
    }

    fn face_target(&mut self) {
        self.facing = 0;
        self.actor.rotation.y = self.actor.rotation.y.wrapping_add(self.sight.angle);
        self.sight.angle = 0;
    }

    fn probability(&self) -> Result<i32> {
        self.rules
            .probability
            .get(self.difficulty)
            .copied()
            .ok_or("Invalid Tenchu 2 AI difficulty".into())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ranged {
    pub attack_delay: u8,
    pub thrown_item: bool,
}

impl Ranged {
    pub fn update(&mut self, context: &mut Context<'_>) -> Result<Decision> {
        if context.sight.distance < 20000 && *context.visibility != Visibility::Hidden {
            *context.visibility = Visibility::Unchanged;
        }
        if context.steering.forced || context.actor.state == State::Item {
            return Ok(Decision::default());
        }
        if context.actor.state == State::Attack {
            return Ok(Decision {
                held: context.combo(20000, 500)?,
                ..Default::default()
            });
        }
        if context.actor.state == State::Jumping {
            return Ok(Decision::default());
        }
        if context.actor.flags & 0x80000 != 0 {
            self.attack_delay = self.attack_delay.wrapping_add(10);
        }
        let mut decision = Decision::default();
        if context.sight.distance < 5500 {
            if context.facing < 1000 {
                decision = context.steer()?;
                decision.held &= input::LEFT | input::RIGHT;
                if (1500..=3000).contains(&context.sight.distance) {
                    if context.random.next15() % 10 == 0 && context.facing < 200 {
                        decision.held = input::ATTACK;
                    }
                } else {
                    decision.held |= input::DOWN;
                }
                self.thrown_item = false;
            } else {
                decision.held = input::UP;
                if context.sight.distance >= 3501
                    && context.actor.inventory.counts[5] != 0
                    && context.random.next15() & 3 == 0
                    && !self.thrown_item
                {
                    decision.held = context.select_item(5)?;
                    self.thrown_item = true;
                }
            }
        } else {
            if context.facing < 200
                && context.difficulty * 10 < usize::from(self.attack_delay)
                && context.random.next15() & 255 < context.probability()?
            {
                let weapon = context
                    .resources
                    .registry
                    .actor(context.actor.kind)
                    .ok_or("Missing Tenchu 2 ranged actor")?
                    .weapon;
                let mask = if context.resources.registry.edition == Edition::Japan
                    && context.target.state == State::Swimming
                {
                    15
                } else {
                    3
                };
                if weapon as u8 != 4 || context.random.next15() & mask == 0 {
                    decision.held = input::ATTACK;
                }
            } else if context.sight.distance > 15000 {
                decision = context.steer()?;
            }
            if context.sight.angle > 200 {
                decision.held = input::RIGHT;
            } else if context.sight.angle < -200 {
                decision.held = input::LEFT;
            } else if context.actor.inventory.counts[6] != 0 && context.random.next15() % 10 == 0 {
                decision.held = context.select_item(6)?;
            } else if context.actor.inventory.counts[10] != 0 && context.random.next15() % 10 == 0 {
                decision.held = context.select_item(10)?;
            }
        }
        if decision.held & input::ATTACK != 0 {
            context.actor.rotation.y = context.actor.rotation.y.wrapping_add(context.sight.angle);
            context.sight.angle = 0;
        }
        Ok(decision)
    }
}
