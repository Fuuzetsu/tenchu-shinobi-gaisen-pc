use std::collections::VecDeque;
use tenchu_core::tenchu2::effects::SparkBurst;

// Keep short-lived events between rendered frames, including fast-forward updates.
#[derive(Default)]
pub struct History {
    frames: VecDeque<(u64, Vec<SparkBurst>)>,
}

impl History {
    pub fn record(&mut self, tick: u64, bursts: &[SparkBurst]) {
        if self
            .frames
            .back()
            .is_some_and(|(previous, _)| previous + 1 != tick)
        {
            self.frames.clear();
        }
        if self.frames.len() == 30 {
            self.frames.pop_front();
        }
        self.frames.push_back((tick, bursts.to_vec()));
    }

    pub fn sparks(&self, tick: u64) -> &[SparkBurst] {
        self.frames
            .iter()
            .find(|(t, _)| *t == tick)
            .map_or(&[], |(_, bursts)| bursts)
    }
}
