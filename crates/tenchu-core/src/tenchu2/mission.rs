use super::{actor::State, ai::runtime::Agent};
use crate::{Result, math::Position};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::campaign::{Kind, Objective, Sequence};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Arrived,
    BossWeakened,
    TargetsDefeated,
    QuotaReached,
    AreaCleared,
    Dead,
    Detected,
}

impl Outcome {
    pub fn success(self) -> bool {
        !matches!(self, Self::Dead | Self::Detected)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Playing {
        pending: Option<Outcome>,
    },
    Ending {
        outcome: Outcome,
        remaining: u16,
        finished_at: u32,
    },
    Finished {
        outcome: Outcome,
        finished_at: u32,
    },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Population {
    pub bosses: u8,
    pub guards: u8,
    pub civilians: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mission {
    pub objective: Objective,
    pub targets: [Option<usize>; 3],
    pub population: Population,
    pub phase: Phase,
    pub next: Option<usize>,
    pub failure: Option<usize>,
    pub results_next: bool,
    pub fade: Option<u8>,
}

#[derive(Clone, Copy)]
pub struct Vitals {
    pub health: i16,
    pub maximum: i16,
}

pub struct Input {
    pub tick: u32,
    pub position: Position,
    pub health: i16,
    pub state: State,
    pub motion_loops: i16,
    pub killed: u8,
    pub detections: u8,
    pub targets: [Option<Vitals>; 3],
}

#[derive(Default)]
pub struct Update {
    pub face_spotter: bool,
    pub began: bool,
    pub detection_sound: bool,
}

impl Mission {
    pub fn new(sequence: Sequence, next: Option<Kind>, player: usize) -> Result<Self> {
        let Kind::Gameplay(objective) = sequence.kind else {
            return Err("Not a Tenchu 2 gameplay sequence".into());
        };
        Ok(Self {
            objective,
            targets: [Some(player), None, None],
            population: Population::default(),
            phase: Phase::Playing { pending: None },
            next: sequence.next,
            failure: sequence.failure,
            results_next: next == Some(Kind::Results),
            fade: None,
        })
    }

    pub fn initialize(&mut self, actors: &[Agent], hero: usize) {
        for (index, agent) in actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.actor.health >= 0)
        {
            let kind = agent.actor.kind;
            if let Objective::Weaken { targets } | Objective::Defeat { targets } = self.objective {
                for (slot, target) in self.targets.iter_mut().zip(targets) {
                    if target != 0 && kind == target {
                        *slot = Some(index);
                    }
                }
            }
            let count = match kind & 0xf0 {
                0 if kind == hero as i16 => continue,
                0 | 0xe0 | 0xf0 => {
                    if matches!(self.objective, Objective::Weaken { .. }) {
                        continue;
                    }
                    &mut self.population.bosses
                }
                0x10 | 0x60 => &mut self.population.civilians,
                0x70 | 0x80 | 0x90 => continue,
                _ => &mut self.population.guards,
            };
            *count = count.wrapping_add(1);
        }
    }

    pub fn input_allowed(&self) -> bool {
        self.phase == Phase::Playing { pending: None }
    }

    pub fn resolve_script(&mut self, outcome: Outcome) -> Result<()> {
        if self.objective != Objective::Scripted {
            return Err("Only scripted missions accept an external outcome".into());
        }
        if let Phase::Playing { pending } = &mut self.phase {
            *pending = Some(outcome);
        }
        Ok(())
    }

    pub fn items_running(&self) -> bool {
        match self.phase {
            Phase::Playing { .. } => true,
            Phase::Ending { remaining, .. } => remaining > 20,
            Phase::Finished { .. } => false,
        }
    }

    fn threshold(&self, outcome: Outcome) -> u16 {
        if outcome.success() && self.results_next {
            16
        } else {
            0
        }
    }

    pub fn update(&mut self, input: Input) -> Update {
        let mut update = Update::default();
        let mut pending = match self.phase {
            Phase::Finished { .. } => return update,
            Phase::Ending {
                mut outcome,
                remaining,
                finished_at,
            } => {
                update.detection_sound =
                    outcome == Outcome::Detected && (input.tick as i32) % 30 == 0;
                let remaining = remaining - 1;
                self.phase = if remaining > self.threshold(outcome) {
                    if remaining < 64 {
                        self.fade = Some(((64 - remaining) * 4) as u8);
                    }
                    Phase::Ending {
                        outcome,
                        remaining,
                        finished_at,
                    }
                } else {
                    if input.health == 0 {
                        outcome = Outcome::Dead;
                    }
                    Phase::Finished {
                        outcome,
                        finished_at,
                    }
                };
                return update;
            }
            Phase::Playing { pending } => pending,
        };
        if self.objective == Objective::Scripted {
            // The imported director owns death scenes as well as success events.
        } else if input.health == 0 || input.tick as i32 > 178200 {
            if input.motion_loops != 0 {
                pending = Some(Outcome::Dead);
            }
        } else {
            match self.objective {
                Objective::Reach { region, unseen } => {
                    if unseen && input.detections != 0 {
                        pending = Some(Outcome::Detected);
                        update.face_spotter = true;
                    }
                    let [x, y, z] = input
                        .position
                        .to_array()
                        .map(|v| i32::from((v / 1000) as i16));
                    let [cx, cy, cz] = region.center.map(i32::from);
                    if input.state != State::Swimming
                        && (y - cy).abs() < 3
                        && (x - cx).abs() <= i32::from(region.radius)
                        && (z - cz).abs() <= i32::from(region.radius)
                    {
                        pending = Some(Outcome::Arrived);
                    }
                }
                Objective::Weaken { .. } => {
                    if input.targets[0].is_some_and(|t| t.health < t.maximum / 2) {
                        pending = Some(Outcome::BossWeakened);
                    }
                }
                Objective::Defeat { .. } => {
                    if input.targets.iter().flatten().all(|t| t.health == 0) {
                        pending = Some(Outcome::TargetsDefeated);
                    }
                }
                Objective::EliminateUnseen { count } => {
                    if i16::from(input.killed) >= count {
                        pending = Some(Outcome::QuotaReached);
                    }
                    if input.detections != 0 {
                        pending = Some(Outcome::Detected);
                        update.face_spotter = true;
                    }
                }
                Objective::EliminateAll => {
                    if u16::from(self.population.bosses) + u16::from(self.population.guards)
                        <= u16::from(input.killed)
                    {
                        pending = Some(Outcome::AreaCleared);
                    }
                }
                Objective::Scripted => unreachable!(),
            }
        }
        self.phase = if let Some(outcome) = pending
            && input.state != State::Attack
        {
            update.began = true;
            let remaining = match outcome {
                Outcome::Detected => 200,
                Outcome::Dead if self.failure.is_none() => 300,
                _ => 100,
            };
            Phase::Ending {
                outcome,
                remaining,
                finished_at: input.tick,
            }
        } else {
            Phase::Playing { pending }
        };
        update
    }
}
