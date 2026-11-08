use super::{Context, Dog, Melee, Ranged};
use crate::{
    Result,
    math::Position,
    tenchu2::{
        actor::{Actor, Resources, State},
        ai::Decision,
        input,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Fighter {
    Unarmed(Melee),
    Blade(Melee),
    Heavy(Melee),
    Ranged(Ranged),
    Dog { state: Dog, attack_delay: u8 },
}

impl Fighter {
    pub fn new(kind: i16, resources: &Resources) -> Result<Self> {
        let weapon = resources
            .registry
            .actor(kind)
            .ok_or("Unknown Tenchu 2 fighter")?
            .weapon;
        Ok(match (weapon as u16 >> 12, kind) {
            (0, 0x1a) => Self::Dog {
                state: Dog::default(),
                attack_delay: 0,
            },
            (0, _) => Self::Unarmed(Melee::default()),
            (1 | 5, _) => Self::Blade(Melee::default()),
            (2 | 3, _) => Self::Heavy(Melee::default()),
            (4, _) => Self::Ranged(Ranged::default()),
            _ => return Err("Unknown Tenchu 2 fighter weapon".into()),
        })
    }

    fn delay(&mut self) -> &mut u8 {
        match self {
            Self::Unarmed(m) | Self::Blade(m) | Self::Heavy(m) => &mut m.attack_delay,
            Self::Ranged(r) => &mut r.attack_delay,
            Self::Dog { attack_delay, .. } => attack_delay,
        }
    }

    fn clear_offset(&mut self) {
        if let Self::Unarmed(m) | Self::Blade(m) | Self::Heavy(m) = self {
            m.offset = Position::default();
        }
    }

    pub fn decide(&mut self, c: &mut Context<'_>) -> Result<Decision> {
        match self {
            Self::Unarmed(m) => m.unarmed(c),
            Self::Blade(m) => m.blade(c),
            Self::Heavy(m) => m.heavy(c),
            Self::Ranged(r) => r.update(c),
            Self::Dog { state, .. } => state.update(c),
        }
    }

    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        tick: i32,
        rotation_phase: &mut i16,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<Decision> {
        if c.actor.kind & 0xe0 == 0xe0 || tick & 3 == 0 {
            *self.delay() = self.delay().wrapping_add(1);
        }
        let mut decision = self.decide(c)?;
        c.apply(&mut decision, &mut apply)?;
        self.finish_attack(c.actor, &mut decision);
        *rotation_phase = rotation_phase.wrapping_add(0x6db) & 4095;
        Ok(decision)
    }

    fn finish_attack(&mut self, actor: &Actor, decision: &mut Decision) {
        if decision.held & input::ATTACK != 0 {
            *self.delay() = 0;
            decision.held &= !(actor.pad.held & input::ATTACK);
        } else if actor.state != State::Attack {
            *self.delay() = self.delay().saturating_add(1);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardHome {
    pub roaming: bool,
    pub return_delay: u8,
    pub fighter: Fighter,
}

impl GuardHome {
    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        home: Position,
        tick: i32,
        rotation_phase: &mut i16,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<Decision> {
        c.keep_visible(10000);
        if c.actor.state == State::Item {
            return Ok(Decision::default());
        }
        if c.actor.state == State::Attack {
            return Ok(Decision {
                held: c.combo(4000, 500)?,
                ..Default::default()
            });
        }
        if matches!(self.fighter, Fighter::Ranged(_)) {
            return self.fighter.update(c, tick, rotation_phase, apply);
        }
        let x = home.x.wrapping_sub(c.actor.position.x);
        let z = home.z.wrapping_sub(c.actor.position.z);
        let distance = c.resources.math.length(Position::new(x, 0, z));
        let mut decision = Decision::default();
        if self.roaming {
            decision = self.fighter.decide(c)?;
            c.apply(&mut decision, &mut apply)?;
            if c.sight.distance < 4000 {
                self.return_delay = self.return_delay.wrapping_add(1);
                if self.return_delay == 0 && distance > 4000 {
                    self.roaming = false;
                }
            }
        } else {
            if c.actor.flags & 0x80000 != 0 {
                self.roaming = true;
            }
            if distance < 2000 {
                if c.actor.motion.frame & 7 == 0 {
                    decision.held = if c.sight.angle > 300 {
                        input::RIGHT
                    } else if c.sight.angle < -300 {
                        input::LEFT
                    } else {
                        if c.random.next15() % 20 == 0 {
                            apply(c.actor, 0x70d)?;
                            self.roaming = true;
                            return Ok(decision);
                        }
                        0
                    };
                } else {
                    decision.held = c.actor.pad.held;
                }
                if c.sight.distance < 5000 {
                    if c.facing < 100 {
                        decision.held |= input::ATTACK;
                    }
                    self.roaming = true;
                }
            } else {
                decision = c.steer_to(x, z)?;
                if decision.held & input::UP != 0
                    && c.actor.flags & 0x2000 == 0
                    && c.steering
                        .ahead
                        .is_some_and(|p| p.material & 0xf00 != 0 || p.height > 4000)
                {
                    self.roaming = true;
                }
                if c.actor.flags & 0x8000 != 0 {
                    self.roaming = true;
                }
                if c.actor.motion.frame == 0 && c.sight.distance < 3000 && c.facing < 1000 {
                    decision.held |= input::ATTACK;
                }
            }
        }
        decision.held &= !(c.actor.pad.held & input::ATTACK);
        Ok(decision)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flank {
    pub engaged: bool,
    pub fighter: Fighter,
}

impl Flank {
    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<Decision> {
        c.keep_visible(10000);
        if c.actor.state == State::Item {
            return Ok(Decision::default());
        }
        if c.actor.state == State::Attack {
            self.fighter.clear_offset();
            self.engaged = false;
            return Ok(Decision {
                held: c.combo(3000, 1300)?,
                ..Default::default()
            });
        }
        let mut decision = Decision::default();
        if self.engaged {
            decision = self.fighter.decide(c)?;
            c.apply(&mut decision, &mut apply)?;
            self.fighter.finish_attack(c.actor, &mut decision);
        } else {
            if c.sight.distance < 4000 {
                decision.held = input::UP;
                if c.facing < 512 {
                    decision.held = if c.sight.angle > 0 {
                        input::LEFT
                    } else {
                        input::RIGHT
                    };
                    if c.sight.distance < 2000 && c.target.state == State::Attack {
                        decision.held = c.guard(decision.held)?;
                    }
                }
                if c.actor.flags & 0x8000 == 0 && i32::from(c.sight.player_angle).abs() <= 1024 {
                    decision.held &= !(c.actor.pad.held & input::ATTACK);
                    return Ok(decision);
                }
            }
            self.engaged = true;
        }
        decision.held &= !(c.actor.pad.held & input::ATTACK);
        Ok(decision)
    }
}
