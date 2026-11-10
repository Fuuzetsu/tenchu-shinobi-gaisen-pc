use super::{
    Decision, ForcedInput, Patrol, PatrolInput, Program, Programs, Sentry, SentryInput, Sleep,
    SleepInput, Wander, WanderInput,
    alert::{self, Confusion, CorpseCue, CorpseInvestigation},
    combat::{Context, Fighter, Flank, GuardHome, Melee},
    perception::{self, Global, Routine, Scene},
    search::{Pursuit, Waiting},
    sight::Visibility,
    status::{AlertCause, Victory},
};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        terrain,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{collision::Collision, layout::Waypoint};

pub trait Events {
    fn action(&mut self, actor: &mut Actor, action: i16, resources: &Resources) -> Result<bool> {
        Ok(actor
            .change_motion(action, Velocity::Motion, resources)?
            .is_some())
    }
    fn voice(&mut self, actor: &Actor, voice: u8) -> Result<()>;
}

pub struct Frame<'a, E> {
    pub terrain: &'a Collision,
    pub cursor: &'a mut terrain::Cursor,
    pub tick: u32,
    pub script: u16,
    pub controllers: [u16; 2],
    pub rotation_phase: &'a mut i16,
    pub events: &'a mut E,
    pub observation: super::bait::Observation<'a>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Active {
    None,
    Sentry(Program, Sentry),
    Wander(Wander),
    Sleep(Sleep),
    Patrol,
    Combat(Fighter),
    Approach(Melee),
    GuardHome(GuardHome),
    Flank(Flank),
    Wait(Waiting),
    Pursue(Pursuit),
    Corpse(CorpseInvestigation),
    Confusion(Confusion),
    Victory(Victory),
}

impl Active {
    fn program(&self) -> Option<Program> {
        Some(match self {
            Self::None | Self::Corpse(_) | Self::Confusion(_) | Self::Victory(_) => return None,
            Self::Sentry(p, _) => *p,
            Self::Wander(_) => Program::Wander,
            Self::Sleep(_) => Program::Sleep,
            Self::Patrol => Program::Patrol,
            Self::Combat(_) => Program::Combat,
            Self::Approach(_) => Program::Approach,
            Self::GuardHome(_) => Program::GuardHome,
            Self::Flank(_) => Program::Flank,
            Self::Wait(_) => Program::WaitForTarget,
            Self::Pursue(_) => Program::Pursue,
        })
    }

    fn new(program: Program, kind: i16, resources: &Resources) -> Result<Self> {
        Ok(match program {
            Program::Disabled
            | Program::Stationary
            | Program::PlayerOne
            | Program::PlayerTwo
            | Program::Investigate
            | Program::Flee => Self::None,
            Program::LookAround | Program::Watch => Self::Sentry(program, Sentry::default()),
            Program::Wander => Self::Wander(Wander::default()),
            Program::Sleep => Self::Sleep(Sleep::default()),
            Program::Patrol => Self::Patrol,
            Program::Combat => Self::Combat(Fighter::new(kind, resources)?),
            Program::Approach => Self::Approach(Melee::default()),
            Program::GuardHome => Self::GuardHome(GuardHome {
                roaming: false,
                return_delay: 0,
                fighter: Fighter::new(kind, resources)?,
            }),
            Program::Flank => Self::Flank(Flank {
                engaged: false,
                fighter: Fighter::new(kind, resources)?,
            }),
            Program::WaitForTarget => Self::Wait(Waiting::default()),
            Program::Pursue => Self::Pursue(Pursuit::default()),
        })
    }

    fn memory(&mut self) -> Option<&mut Position> {
        fn fighter_memory(fighter: &mut Fighter) -> Option<&mut Position> {
            match fighter {
                Fighter::Unarmed(m) | Fighter::Blade(m) | Fighter::Heavy(m) => Some(&mut m.offset),
                _ => None,
            }
        }
        match self {
            Self::Combat(fighter) => fighter_memory(fighter),
            Self::GuardHome(state) => fighter_memory(&mut state.fighter),
            Self::Flank(state) => fighter_memory(&mut state.fighter),
            Self::Approach(state) => Some(&mut state.offset),
            Self::Pursue(state) => Some(&mut state.remembered),
            Self::Corpse(state) => Some(&mut state.position),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Behavior {
    pub bait: Option<super::bait::Bait>,
    pub programs: Programs,
    layout_idle: Program,
    pub home: Position,
    pub patrol: Patrol,
    pub forced: Option<ForcedInput>,
    pub has_suspicious_program: bool,
    pub recovering_confusion: bool,
    active: Active,
    memory: Position,
    remembered_visibility: Visibility,
}

impl Behavior {
    pub fn new(home: Position, route: &[Waypoint]) -> Self {
        Self {
            bait: None,
            programs: Programs::default(),
            layout_idle: Program::Disabled,
            home,
            patrol: Patrol::new(route, home),
            forced: None,
            has_suspicious_program: false,
            recovering_confusion: false,
            active: Active::None,
            memory: Position::default(),
            remembered_visibility: Visibility::Unchanged,
        }
    }

    pub fn assign(
        &mut self,
        code: u16,
        map: usize,
        difficulty: u8,
        random: &mut Random,
    ) -> Result<bool> {
        let enabled = self.programs.assign(code, map, difficulty, random)?;
        self.layout_idle = Program::from_code(code & 15)?;
        self.has_suspicious_program |= enabled;
        Ok(enabled)
    }

    pub fn release_bait(&mut self, actor: &mut Actor, player: usize) {
        self.bait = None;
        self.programs.idle = self.layout_idle;
        actor.target = Some(player);
    }

    pub(crate) fn relocate(&mut self, home: Position) {
        let mut programs = self.programs.clone();
        programs.idle = if self.layout_idle == Program::Patrol {
            Program::LookAround
        } else {
            self.layout_idle
        };
        *self = Self {
            layout_idle: programs.idle,
            programs,
            has_suspicious_program: self.has_suspicious_program,
            ..Self::new(home, &[])
        };
    }

    pub fn reset(&mut self, forget: bool) {
        self.active = Active::None;
        self.patrol.turning = 0;
        self.recovering_confusion = false;
        if forget {
            self.memory = Position::default();
        }
    }

    pub fn remember(&mut self, position: Position) {
        self.memory = position;
        if let Some(memory) = self.active.memory() {
            *memory = position;
        }
    }

    pub fn observe(&mut self, visibility: Visibility) {
        self.remembered_visibility = visibility;
    }

    fn accept(&mut self, c: &mut Context<'_>, decision: &Decision) {
        if let Some(forced) = decision.forced {
            self.forced = Some(forced);
        }
        c.steering.forced = self.forced.is_some();
        if let Some(memory) = self.active.memory() {
            self.memory = *memory;
        }
    }

    pub fn perceive(
        &mut self,
        c: &mut Context<'_>,
        scene: &Scene<'_>,
        global: &mut Global,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<perception::Outcome> {
        let outcome = perception::update(
            c,
            scene,
            global,
            &mut Reactions {
                behavior: self,
                frame,
            },
        )?;
        self.accept(c, &outcome.decision);
        Ok(outcome)
    }

    pub fn decide(
        &mut self,
        program: Program,
        c: &mut Context<'_>,
        global: &mut Global,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<Decision> {
        let immediate = match program {
            Program::Disabled | Program::Stationary => Some(Decision::default()),
            Program::PlayerOne | Program::PlayerTwo => Some(Decision {
                held: frame.controllers[usize::from(program == Program::PlayerTwo)],
                ..Default::default()
            }),
            Program::Investigate => Some(alert::investigate(c, self.remembered_visibility)?),
            Program::Flee => Some(alert::flee(c)?),
            _ => None,
        };
        if let Some(decision) = immediate {
            self.accept(c, &decision);
            return Ok(decision);
        }
        if self.active.program() != Some(program) {
            self.active = Active::new(program, c.actor.kind, c.resources)?;
            if let Some(memory) = self.active.memory() {
                *memory = self.memory;
            }
        }
        let resources = c.resources;
        let mut apply =
            |actor: &mut Actor, action| frame.events.action(actor, action, resources).map(|_| ());
        let mut finished = false;
        let mut decision = match &mut self.active {
            Active::Sentry(Program::Watch, sentry) => {
                alert::watch(sentry, c, frame.script, global.alert.position, &mut apply)?
            }
            Active::Sentry(_, sentry) => {
                let (held, action) = sentry.update(
                    SentryInput {
                        action: c.actor.motion.action,
                        health: c.actor.health,
                        turn: c.actor.turn,
                        held: c.actor.pad.held,
                    },
                    c.random,
                )?;
                Decision {
                    held,
                    action,
                    ..Default::default()
                }
            }
            Active::Wander(wander) => wander.update(
                &WanderInput {
                    position: c.actor.position,
                    home: self.home,
                    action: c.actor.motion.action,
                    health: c.actor.health,
                    loops: c.actor.motion.loops,
                    in_water: c.actor.flags & 0x2000 != 0,
                    steering: c.steering,
                },
                &mut c.sight.destination,
                &resources.math,
                c.random,
            )?,
            Active::Sleep(sleep) => {
                let output = sleep.update(
                    c.actor,
                    SleepInput {
                        home: self.home,
                        alert: global.alert.remaining,
                    },
                    &mut c.sight.destination,
                    c.random,
                    &mut apply,
                )?;
                if output.unnoticed {
                    *c.visibility = Visibility::Unnoticed;
                    self.remembered_visibility = Visibility::Unnoticed;
                }
                Decision {
                    held: output.held,
                    ..Default::default()
                }
            }
            Active::Patrol => {
                if c.steering.forced {
                    Decision::default()
                } else {
                    if c.actor.weapon_drawn() && c.actor.health > 0 {
                        apply(c.actor, 0x701)?;
                    }
                    self.patrol.update(
                        &PatrolInput {
                            position: c.actor.position,
                            yaw: c.actor.rotation.y,
                            turn: c.actor.turn,
                            action: c.actor.motion.action,
                            health: c.actor.health,
                            held: c.actor.pad.held,
                            blocked: c.actor.flags & 0x48000 != 0,
                            touching_player: c.actor.flags & 0x400000 != 0,
                            in_water: c.actor.flags & 0x2000 != 0,
                            ahead: c.steering.ahead,
                        },
                        &resources.math,
                        c.random,
                    )?
                }
            }
            Active::Combat(fighter) => {
                fighter.update(c, frame.tick as i32, frame.rotation_phase, &mut apply)?
            }
            Active::Approach(melee) => melee.approach_combat(c, &mut apply)?,
            Active::GuardHome(guard) => guard.update(
                c,
                self.home,
                frame.tick as i32,
                frame.rotation_phase,
                &mut apply,
            )?,
            Active::Flank(flank) => flank.update(c, &mut apply)?,
            Active::Wait(waiting) => {
                let result = waiting.update(
                    c,
                    &mut global.alert.remaining,
                    self.programs.searching == Program::WaitForTarget,
                    &mut apply,
                )?;
                finished = result.finished;
                result.decision
            }
            Active::Pursue(pursuit) => {
                let result = pursuit.update(
                    c,
                    &mut global.alert.remaining,
                    frame.terrain,
                    frame.cursor,
                    &mut apply,
                )?;
                finished = result.finished;
                result.decision
            }
            _ => unreachable!("Program selection produces a program state"),
        };
        if let Some(action) = decision.action.take() {
            apply(c.actor, action)?;
        }
        self.accept(c, &decision);
        if finished {
            self.reset(true);
        }
        Ok(decision)
    }

    pub fn idle(
        &mut self,
        c: &mut Context<'_>,
        global: &mut Global,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<Decision> {
        if let Some(bait) = self.bait {
            let decision = match super::bait::update(bait.position, c, global, frame)? {
                super::bait::Outcome::Continue(decision) => decision,
                super::bait::Outcome::ResumePatrol => {
                    self.release_bait(c.actor, frame.observation.player);
                    self.reset(false);
                    Decision::default()
                }
                super::bait::Outcome::LookAround => {
                    return self.decide(Program::LookAround, c, global, frame);
                }
            };
            self.accept(c, &decision);
            return Ok(decision);
        }
        self.decide(self.programs.idle, c, global, frame)
    }

    pub fn confusion(&mut self, c: &mut Context<'_>) -> Result<Decision> {
        if c.actor.state == State::Attack {
            return Confusion::default().update(c);
        }
        if !matches!(self.active, Active::Confusion(_)) {
            self.active = Active::Confusion(Confusion::default());
        }
        let Active::Confusion(confusion) = &mut self.active else {
            unreachable!()
        };
        let decision = confusion.update(c)?;
        self.accept(c, &decision);
        Ok(decision)
    }

    pub fn victory(&mut self, c: &mut Context<'_>, events: &mut impl Events) -> Result<()> {
        if c.actor.health <= 0 {
            return Ok(());
        }
        if !matches!(self.active, Active::Victory(_)) {
            self.active = Active::Victory(Victory::default());
        }
        let Active::Victory(victory) = &mut self.active else {
            unreachable!()
        };
        victory.update(c.actor, c.random, |actor, action| {
            events.action(actor, action, c.resources).map(|_| ())
        })
    }

    pub fn corpse(
        &mut self,
        c: &mut Context<'_>,
        global: &mut Global,
        events: &mut impl Events,
    ) -> Result<Decision> {
        if !matches!(self.active, Active::Corpse(_)) {
            self.active = Active::Corpse(CorpseInvestigation {
                position: self.memory,
                ..Default::default()
            });
        }
        let Active::Corpse(corpse) = &mut self.active else {
            unreachable!()
        };
        let (resources, difficulty, rules) = (c.resources, c.difficulty as u8, c.rules);
        let decision = corpse.update(
            c,
            &mut self.home,
            &mut self.programs.idle,
            |actor, _, cue| match cue {
                CorpseCue::DrawWeapon => events.action(actor, 0x700, resources).map(|_| ()),
                CorpseCue::SharedAlert => global.alert.raise(AlertCause::Combat, difficulty, rules),
                CorpseCue::Voice => events.voice(actor, 9),
            },
        )?;
        self.accept(c, &decision);
        Ok(decision)
    }
}

struct Reactions<'a, 'b, E> {
    behavior: &'a mut Behavior,
    frame: &'a mut Frame<'b, E>,
}

impl<E: Events> perception::Reactions for Reactions<'_, '_, E> {
    fn decide(
        &mut self,
        routine: Routine,
        c: &mut Context<'_>,
        global: &mut Global,
    ) -> Result<Decision> {
        let program = match routine {
            Routine::Idle => return self.behavior.idle(c, global, self.frame),
            Routine::Suspicious => self.behavior.programs.suspicious,
            Routine::Combat => self.behavior.programs.combat,
            Routine::Search => self.behavior.programs.searching,
            Routine::Watch => Program::Watch,
            Routine::Corpse => return self.behavior.corpse(c, global, self.frame.events),
        };
        self.behavior.decide(program, c, global, self.frame)
    }
    fn action(&mut self, c: &mut Context<'_>, action: i16) -> Result<bool> {
        self.frame.events.action(c.actor, action, c.resources)
    }
    fn voice(&mut self, c: &mut Context<'_>, voice: u8) -> Result<()> {
        self.frame.events.voice(c.actor, voice)
    }
    fn reset(&mut self, forget: bool) {
        self.behavior.reset(forget);
    }
    fn remember(&mut self, position: Position) {
        self.behavior.remember(position);
    }
    fn observe(&mut self, visibility: Visibility) {
        self.behavior.observe(visibility);
    }
}
