use serde::Serialize;
use std::collections::VecDeque;

#[derive(Serialize)]
pub struct Recent<T> {
    scene: Option<u64>,
    #[serde(skip)]
    tick: Option<u64>,
    pub inputs: VecDeque<T>,
}

impl<T> Default for Recent<T> {
    fn default() -> Self {
        Self {
            scene: None,
            tick: None,
            inputs: VecDeque::new(),
        }
    }
}

impl<T> Recent<T> {
    pub fn reset(&mut self, scene: u64) {
        self.inputs.clear();
        self.scene = Some(scene);
        self.tick = None;
    }

    pub fn push(&mut self, scene: u64, tick: u64, input: T) {
        if self.scene != Some(scene) || self.tick.is_some_and(|before| before >= tick) {
            self.reset(scene);
        }
        if self.inputs.len() == 600 {
            self.inputs.pop_front();
        }
        self.inputs.push_back(input);
        self.tick = Some(tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_history_drops_stale_scene_and_rewind_inputs() {
        let mut recent = Recent::default();
        for tick in 0..605 {
            recent.push(3, tick, tick);
        }
        assert_eq!(recent.inputs.len(), 600);
        assert_eq!(recent.inputs.front(), Some(&5));
        assert_eq!(recent.inputs.back(), Some(&604));
        recent.push(3, 100, 100);
        assert_eq!(recent.inputs.len(), 1);
        recent.push(4, 101, 101);
        assert_eq!(recent.inputs.len(), 1);
        recent.reset(5);
        assert!(recent.inputs.is_empty());
    }
}
