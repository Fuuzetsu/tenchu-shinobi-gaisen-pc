use super::project::{Action, Event, Trigger};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Runtime {
    last_tick: Option<u64>,
    started: Vec<bool>,
    inside: Vec<bool>,
    pending: Vec<(u64, usize, usize)>,
    pub flags: BTreeMap<String, bool>,
}

pub struct Observation<'a> {
    pub tick: u64,
    pub player: [i32; 3],
    pub defeated: &'a [bool],
}

impl Runtime {
    pub fn step(&mut self, events: &[Event], state: Observation<'_>) -> Vec<Action> {
        if self.last_tick.is_some_and(|tick| state.tick <= tick) {
            return vec![];
        }
        self.last_tick = Some(state.tick);
        self.started.resize(events.len(), false);
        self.inside.resize(events.len(), false);
        for (index, event) in events.iter().enumerate() {
            let matches = event.enabled
                && match &event.trigger {
                    Trigger::Start => true,
                    Trigger::AfterTicks(tick) => state.tick >= u64::from(*tick),
                    Trigger::Enter { min, max } => {
                        (0..3).all(|i| (min[i]..=max[i]).contains(&state.player[i]))
                    }
                    Trigger::Defeated(slot) => state.defeated.get(*slot).copied().unwrap_or(false),
                    Trigger::Flag { name, value } => {
                        self.flags.get(name).copied().unwrap_or(false) == *value
                    }
                };
            if matches
                && !self.inside[index]
                && !(event.once && self.started[index])
                && !self.pending.iter().any(|(_, pending, _)| *pending == index)
            {
                self.started[index] = true;
                let mut due = state.tick;
                for (cue, action) in event.cues.iter().enumerate() {
                    due = due.saturating_add(u64::from(action.delay));
                    self.pending.push((due, index, cue));
                }
            }
            self.inside[index] = matches;
        }
        self.pending.sort_unstable();
        let due = self
            .pending
            .partition_point(|(tick, _, _)| *tick <= state.tick);
        let actions: Vec<_> = self
            .pending
            .drain(..due)
            .filter_map(|(_, event, cue)| {
                events
                    .get(event)
                    .filter(|e| e.enabled)
                    .and_then(|e| e.cues.get(cue))
                    .map(|c| c.action.clone())
            })
            .collect();
        for action in &actions {
            if let Action::Flag { name, value } = action {
                self.flags.insert(name.clone(), *value);
            }
        }
        actions
    }
}
