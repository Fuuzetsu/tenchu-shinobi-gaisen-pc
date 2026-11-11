use super::{
    Decision, Program, alert,
    behavior::{Behavior, Events, Frame},
    combat::Context,
};
use crate::{
    Result,
    ai::{
        Recruitment,
        profiles::{Combat, Idle, Overrides},
    },
    tenchu2::{actor::State, input},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub combat: Combat,
    pub idle: Idle,
    pub actor: usize,
    pub seed: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub overrides: Overrides,
    pub protected: bool,
    pub selection: Option<Selection>,
    pub recruitment: Recruitment,
    pub recruited_by: Option<usize>,
    pub(super) last_attack: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reinforcements {
    pub settings: crate::roster::ReinforcementCall,
    pub limit: Option<usize>,
    pub alive: Vec<bool>,
}

impl Behavior {
    pub fn select_profile(&mut self, selection: Option<Selection>) {
        if self.profile.selection != selection {
            self.profile.selection = selection;
            self.profile.last_attack = None;
            self.reset(false);
        }
    }

    pub(super) fn profile_idle(
        &mut self,
        c: &mut Context<'_>,
        global: &mut super::perception::Global,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<Option<Decision>> {
        let Some(selection) = self.profile.selection else {
            return Ok(None);
        };
        if self.programs.idle == Program::Sleep
            || !matches!(c.actor.state, State::Normal | State::Ready)
            || c.steering.forced
        {
            return Ok(None);
        }
        let decision = match selection.idle {
            Idle::Original => return Ok(None),
            Idle::Patrol if self.patrol.waypoints().next().is_some() => {
                self.decide(Program::Patrol, c, global, frame)?
            }
            Idle::Patrol | Idle::Lookout => self.decide(Program::LookAround, c, global, frame)?,
            Idle::TracePlayer => {
                let delta = frame
                    .observation
                    .body
                    .position
                    .wrapping_sub(c.actor.position);
                let angle =
                    super::direction(&c.resources.math, delta.x, delta.z, c.actor.rotation.y)?;
                Decision {
                    held: if angle > c.actor.turn {
                        input::RIGHT
                    } else if i32::from(angle) < -i32::from(c.actor.turn) {
                        input::LEFT
                    } else {
                        0
                    },
                    ..Default::default()
                }
            }
        };
        Ok(Some(decision))
    }

    pub(super) fn profile_combat(
        &mut self,
        c: &mut Context<'_>,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<Option<Decision>> {
        let Some(selection) = self.profile.selection else {
            return Ok(None);
        };
        let mut profile = selection.combat;
        if profile == Combat::Original {
            return Ok(None);
        }
        if !matches!(c.actor.state, State::Normal | State::Ready | State::Attack) {
            return Ok(Some(Decision::default()));
        }
        let category = c.category()?;
        let reach = i32::from(c.rules.reach[category]);
        let target = c.target()?;
        if profile == Combat::Watchman {
            profile = if c.actor.health < c.actor.max_health / 3 || target.state == State::Attack {
                Combat::Cautious
            } else if c.sight.distance > reach + 2000 {
                Combat::Reckless
            } else {
                Combat::Flanker
            };
        }
        let phase = frame
            .tick
            .wrapping_add((selection.actor as u32).wrapping_mul(13))
            % 90;
        if c.actor.state == State::Attack {
            let follow = match profile {
                Combat::Coward => false,
                Combat::Cautious => target.state != State::Attack && phase < 30,
                Combat::Flanker => phase >= 45,
                _ => true,
            };
            return Ok(Some(Decision {
                held: if follow { c.combo(reach, 600)? } else { 0 },
                ..Default::default()
            }));
        }
        let decision = match profile {
            Combat::Coward => {
                if self.profile.recruitment == Recruitment::Available {
                    self.profile.recruitment = Recruitment::Requested;
                }
                alert::flee(c)?
            }
            Combat::Reckless => self.profile_strike(c, frame.tick, reach)?,
            Combat::Cautious if target.state != State::Attack && phase < 30 => {
                self.profile_strike(c, frame.tick, reach)?
            }
            Combat::Cautious => {
                let mut decision = c.steer()?;
                decision.held &= input::LEFT | input::RIGHT;
                if c.sight.distance < reach + 500 {
                    decision.held |= input::DOWN;
                }
                decision
            }
            Combat::Flanker if c.sight.distance > reach + 1000 || phase >= 45 => {
                self.profile_strike(c, frame.tick, reach)?
            }
            Combat::Flanker => {
                if c.actor.state == State::Ready
                    && matches!(c.actor.motion.action, 0x100..=0x104)
                    && c.facing < 700
                    && phase == 0
                {
                    let action = if (selection.actor as u16 ^ selection.seed) & 1 == 0 {
                        0x113
                    } else {
                        0x114
                    };
                    let clear = if action == 0x113 {
                        c.steering.right
                    } else {
                        c.steering.left
                    };
                    if clear
                        && c.resources
                            .registry
                            .clip(c.actor.definition_kind(), action)
                            .is_some_and(|clip| c.resources.clips.contains_key(&clip))
                    {
                        frame.events.action(c.actor, action, c.resources)?;
                        return Ok(Some(Decision::default()));
                    }
                }
                let mut decision = c.steer()?;
                decision.held &= input::LEFT | input::RIGHT;
                decision
            }
            Combat::Original | Combat::Mixed | Combat::Watchman => unreachable!(),
        };
        Ok(Some(decision))
    }

    fn profile_strike(&mut self, c: &mut Context<'_>, tick: u32, reach: i32) -> Result<Decision> {
        let mut decision = c.steer()?;
        decision.held &= input::LEFT | input::RIGHT;
        if c.sight.distance < reach
            && c.facing < 500
            && self
                .profile
                .last_attack
                .is_none_or(|at| tick.wrapping_sub(at) > c.difficulty as u32 * 10)
        {
            decision.held |= input::ATTACK & !c.actor.pad.held;
            if decision.held & input::ATTACK != 0 {
                self.profile.last_attack = Some(tick);
            }
        } else if c.sight.distance >= reach {
            decision.held |= input::UP;
        }
        Ok(decision)
    }
}
