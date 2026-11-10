use std::collections::VecDeque;
use tenchu_core::tenchu2::effects::{Contact, Frame};
pub(super) mod blood;

// Keep short-lived events between rendered frames, including fast-forward updates.
#[derive(Default)]
pub struct History {
    frames: VecDeque<(u64, Vec<Contact>)>,
    pub blood: blood::History,
}

impl History {
    pub fn record(&mut self, tick: u64, frame: &Frame) {
        if self
            .frames
            .back()
            .is_some_and(|(previous, _)| previous + 1 != tick)
        {
            self.frames.clear();
            self.blood.reset();
        }
        if self.frames.len() == 30 {
            self.frames.pop_front();
        }
        self.frames.push_back((tick, frame.contacts.clone()));
        self.blood.record(&frame.quads);
    }

    pub fn events(&self, tick: u64) -> &[Contact] {
        self.frames
            .iter()
            .find(|(t, _)| *t == tick)
            .map_or(&[], |(_, bursts)| bursts)
    }
}
