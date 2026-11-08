use super::{
    Decision, ForcedInput, Program, Sentry, SentryInput, combat::Context, local_alert_only,
    sight::Visibility,
};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, State},
        input,
    },
};
use serde::{Deserialize, Serialize};

pub fn watch(
    sentry: &mut Sentry,
    c: &mut Context<'_>,
    script_mode: u16,
    shared_position: Option<Position>,
    mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
) -> Result<Decision> {
    let mut decision = if script_mode < 2 {
        let delta = shared_position
            .map(|p| p.wrapping_sub(c.actor.position))
            .unwrap_or_default();
        let mut decision = c.steer_to(delta.x, delta.z)?;
        decision.held &= input::LEFT | input::RIGHT;
        decision
    } else {
        let (held, action) = sentry.update(
            c.resources.registry.edition,
            SentryInput {
                action: c.actor.motion.action,
                health: c.actor.health,
                turn: c.actor.turn,
                held: c.actor.pad.held,
            },
            c.random,
        )?;
        if let Some(action) = action {
            apply(c.actor, action)?;
        }
        Decision {
            held,
            ..Default::default()
        }
    };
    if *sentry == Sentry::Begin
        && c.sight.distance < 16000
        && c.facing < 300
        && (c.actor.motion.loops == 1 || c.actor.motion.loops & 8 != 0)
        && c.difficulty == 1
    {
        let item = if c.random.next15() & 1 != 0 { 4 } else { 8 };
        if c.actor.inventory.counts[item] != 0 && c.random.next15() % (c.difficulty as i32 + 1) == 0
        {
            decision.held = c.select_item(item as u8)?;
            *sentry = Sentry::Waiting { remaining: 0 };
        }
    }
    Ok(decision)
}

pub fn investigate(c: &mut Context<'_>, remembered: Visibility) -> Result<Decision> {
    let mut forced = None;
    if c.observed_flags & 0x8000 != 0 && !c.steering.forced {
        let held = if c.sight.angle > 0 {
            input::RIGHT
        } else {
            input::LEFT
        };
        forced = Some(ForcedInput::turn(held, 512, c.actor.turn, 1)?);
        c.steering.forced = true;
    }
    let mut decision = match remembered {
        Visibility::Unchanged => Decision::default(),
        Visibility::Hidden | Visibility::Unnoticed => {
            let delta = c.sight.destination.wrapping_sub(c.actor.position);
            c.steer_to(delta.x, delta.z)?
        }
        Visibility::Recognized | Visibility::Suspicious => c.steer()?,
    };
    decision.forced = decision.forced.or(forced);
    Ok(decision)
}

pub fn flee(c: &mut Context<'_>) -> Result<Decision> {
    if c.steering.forced {
        return Ok(Decision::default());
    }
    c.keep_visible(15000);
    let mut held = if c.sight.angle > 0 {
        input::LEFT
    } else if c.sight.angle < 0 {
        input::RIGHT
    } else {
        0
    };
    held |= if c.facing < 1000 {
        input::DOWN
    } else {
        input::UP
    };
    let forced = if c.actor.flags & 0x8000 != 0 && c.facing > 1000 {
        Some(ForcedInput::turn(
            if c.sight.angle > 0 {
                input::LEFT
            } else {
                input::RIGHT
            },
            1024,
            c.actor.turn,
            0,
        )?)
    } else {
        None
    };
    Ok(Decision {
        held,
        forced,
        ..Default::default()
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfusionPhase {
    #[default]
    Right,
    Retreat,
    Left,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Confusion {
    pub phase: ConfusionPhase,
    pub started: bool,
    pub remaining: u8,
}

impl Confusion {
    pub fn update(&mut self, c: &mut Context<'_>) -> Result<Decision> {
        if c.actor.state == State::Attack {
            return Ok(Decision {
                held: c.combo(i32::MAX, 2047)?,
                ..Default::default()
            });
        }
        let held = if self.started {
            match self.phase {
                ConfusionPhase::Right => {
                    self.phase = ConfusionPhase::Retreat;
                    self.started = false;
                }
                ConfusionPhase::Retreat => {
                    self.remaining = self.remaining.wrapping_sub(1);
                    if self.remaining == 0 {
                        self.phase = ConfusionPhase::Left;
                        self.started = false;
                    }
                }
                ConfusionPhase::Left => {
                    self.phase = ConfusionPhase::Right;
                    self.started = false;
                }
            }
            c.actor.pad.held
        } else {
            self.started = true;
            match self.phase {
                ConfusionPhase::Retreat => {
                    self.remaining = 30;
                    input::DOWN | input::RIGHT
                }
                _ if c.category()? == 4 => input::ATTACK,
                ConfusionPhase::Right => input::ATTACK | input::RIGHT,
                ConfusionPhase::Left => input::ATTACK | input::LEFT,
            }
        };
        Ok(Decision {
            held: held & !(c.actor.pad.held & input::ATTACK),
            ..Default::default()
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorpseCue {
    DrawWeapon,
    SharedAlert,
    Voice,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorpseInvestigation {
    pub position: Position,
    pub arrived: bool,
    pub elapsed: i16,
    pub turning: u8,
}

impl CorpseInvestigation {
    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        home: &mut Position,
        idle: &mut Program,
        mut cue: impl FnMut(&mut Actor, &mut Random, CorpseCue) -> Result<()>,
    ) -> Result<Decision> {
        if self.arrived {
            let held = if self.turning != 0 {
                self.turning = self.turning.wrapping_sub(1);
                c.actor.pad.held
            } else if c.actor.motion.loops == 1 && c.actor.pad.held == 0 {
                let held = if c.random.next15() & 1 == 0 {
                    input::LEFT
                } else {
                    input::RIGHT
                };
                self.turning = (1024i32
                    .checked_div(i32::from(c.actor.turn))
                    .ok_or("Tenchu 2 corpse observer cannot turn at zero speed")?
                    as u8)
                    .wrapping_add(1);
                held
            } else {
                0
            };
            return Ok(Decision {
                held,
                ..Default::default()
            });
        }
        let delta = self.position.wrapping_sub(c.actor.position);
        if !c.actor.weapon_drawn() {
            cue(c.actor, c.random, CorpseCue::DrawWeapon)?;
        }
        let mut decision = c.steer_to(delta.x, delta.z)?;
        if (c.observed_flags & 0x48000 != 0 && decision.held & (input::LEFT | input::RIGHT) != 0)
            || c.facing < 300
        {
            decision.held |= input::UP;
        }
        if c.resources.math.length(Position::new(delta.x, 0, delta.z)) < 2000 {
            if !local_alert_only(c.actor.kind) {
                cue(c.actor, c.random, CorpseCue::SharedAlert)?;
            }
            cue(c.actor, c.random, CorpseCue::Voice)?;
            self.arrived = true;
        } else if self.elapsed < 300 {
            self.elapsed = self.elapsed.wrapping_add(1);
            return Ok(decision);
        }
        *home = c.actor.position;
        *idle = if c.random.next15() & 1 == 0 {
            Program::LookAround
        } else {
            Program::Wander
        };
        Ok(decision)
    }
}
