use super::Game;
use crate::actor::State as ActorState;
use crate::{Result, camera, roster::Roster};
use tenchu_assets::script::Event;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Outcome {
    #[default]
    Running,
    Complete,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    event: usize,
    target: usize,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    event: usize,
    target_kind: i16,
    clock: i32,
}

#[derive(Debug, Default)]
pub struct Events {
    pub time: i32,
    pub outcome: Outcome,
    slots: [Option<Slot>; 2],
    pending: Option<Pending>,
    dying: bool,
    enabled: bool,
}

impl Events {
    pub(super) fn new(events: &[Event], roster: &Roster, player: usize) -> Self {
        let mut state = Self {
            enabled: !events.is_empty(),
            ..Self::default()
        };
        state.update(0, 0, events, roster, player);
        state
    }

    fn update(&mut self, slot: usize, id: u8, events: &[Event], roster: &Roster, player: usize) {
        self.slots[slot] = None;
        if id == 255 {
            return;
        }
        let Some(event) = events.iter().position(|e| e.id == id) else {
            return;
        };
        let target = if events[event].target == 255 {
            Some(player)
        } else {
            roster.find(i16::from(events[event].target))
        };
        if let Some(target) = target {
            let actor = roster.get(target);
            if !(actor.status == ActorState::Dead && actor.motion.loops == -1)
                && (!(2..=3).contains(&id) || actor.health > 0)
            {
                self.slots[slot] = Some(Slot { event, target });
            }
        }
    }
}

fn triggered(
    event: &Event,
    target: &crate::actor::Actor,
    player: &crate::actor::Actor,
    time: i32,
) -> bool {
    match event.mode {
        0 | 8 => true,
        1 => {
            let target = if (2..=3).contains(&event.id) {
                player
            } else {
                target
            };
            [0, 2, 1].into_iter().all(|i| {
                let value = (target.position[i] / 1000) as i16;
                (event.bounds[i][0]..=event.bounds[i][1]).contains(&value)
            })
        }
        2 => target.attribute & event.value as u16 == event.value as u16,
        3 => player.status != ActorState::Attack && target.status.source_id() == event.value,
        4 => target.motion.action == event.value,
        5 => target.health <= event.value,
        6 => [1, 0, 2].into_iter().all(|i| {
            player.position[i]
                .wrapping_sub(target.position[i])
                .wrapping_abs()
                <= 2000
        }),
        7 => event.value >= 0 && time >= i32::from(event.value),
        _ => false,
    }
}

impl Game {
    fn update_event(&mut self, slot: usize, id: u8) {
        self.events.update(
            slot,
            id,
            &self.mission.scripts.events,
            &self.roster,
            self.player_id,
        );
    }

    fn event_handoff(&mut self, pending: Pending) -> bool {
        let rikimaru = self.player().kind == 0;
        let event = &mut self.mission.scripts.events[pending.event];
        if rikimaru && event.mode == 5 && pending.target_kind == 0x8d {
            event.next[0] = 100;
        }
        let next = event.next;
        let complete = (1..=3).contains(&event.id);
        self.events.time = 0;
        self.clock = pending.clock;
        self.update_event(0, next[0]);
        self.update_event(1, next[1]);
        if complete {
            self.events.outcome = if self.events.dying {
                Outcome::GameOver
            } else {
                Outcome::Complete
            };
            self.events.dying = false;
        }
        complete
    }

    fn death_wait(&mut self) -> Result<()> {
        self.events.time = -100;
        self.events.slots = [None; 2];
        self.events.dying = false;
        if self.camera.mode != camera::Mode::Fall {
            self.camera.set_mode(
                camera::Mode::Critical,
                self.roster.get(self.player_id),
                &mut camera::Environment {
                    terrain: &self.mission.terrain,
                    cursor: &mut self.cursor,
                    math: &self.math,
                    body: &self.body,
                },
                &mut self.random,
            )?;
        }
        self.intelligence.action_halt = crate::ai::Halt::StageEnd;
        self.roster.get_mut(self.player_id).status = ActorState::Dead;
        Ok(())
    }

    pub(super) fn stage_sequence(&mut self) -> Result<()> {
        if !self.events.enabled || self.cinematic.active || self.events.outcome != Outcome::Running
        {
            return Ok(());
        }
        if let Some(pending) = self.events.pending.take()
            && self.event_handoff(pending)
        {
            return Ok(());
        }
        for _ in 0..4096 {
            if self.player().status == ActorState::Dead {
                if self.player().motion.loops == 0 && self.player().motion.count < 30 {
                    return Ok(());
                }
                if self.events.slots.iter().all(Option::is_none) {
                    if self.player().motion.loops != 0 {
                        self.events.time = self.events.time.wrapping_add(1);
                    }
                    if self.events.time >= 0 {
                        self.events.outcome = Outcome::GameOver;
                    }
                    return Ok(());
                }
                self.update_event(0, 2);
                self.update_event(1, 3);
                self.roster.get_mut(self.player_id).status = ActorState::Action;
                self.events.dying = true;
            }
            self.events.time = self.events.time.wrapping_add(1);
            let mut handoff = false;
            for i in 0..2 {
                let Some(slot) = self.events.slots[i] else {
                    continue;
                };
                let event = &self.mission.scripts.events[slot.event];
                if !(2..=3).contains(&event.id) && self.player().health == 0 {
                    continue;
                }
                if !triggered(
                    event,
                    self.roster.get(slot.target),
                    self.player(),
                    self.events.time,
                ) {
                    continue;
                }
                if event.mode == 8 {
                    self.music.push(Some(event.value));
                }
                let pending = Pending {
                    event: slot.event,
                    target_kind: self
                        .roster
                        .get(slot.target)
                        .script_kind
                        .unwrap_or(self.roster.get(slot.target).kind),
                    clock: self.clock,
                };
                if event.sequence != 255 {
                    let mut sequence = i16::from(event.sequence);
                    if sequence == 0 && self.mission.stage.id == 8 {
                        sequence = 5 - self.score().grade;
                    }
                    if self.begin_cinematic(sequence)? {
                        self.events.pending = Some(pending);
                        return Ok(());
                    }
                    self.events.slots[i] = None;
                    continue;
                }
                if self.event_handoff(pending) {
                    return Ok(());
                }
                handoff = true;
                break;
            }
            if !handoff {
                if self.events.dying {
                    self.death_wait()?;
                }
                return Ok(());
            }
        }
        Err("Stage events form a non-yielding cycle".into())
    }
}
