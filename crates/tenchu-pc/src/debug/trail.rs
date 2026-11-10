use super::{Debug, overlay::Builder};
use glam::Vec3;

impl Debug {
    pub(crate) fn clear_trail(&mut self) {
        self.trail.clear();
        self.last_tick = 0;
    }

    pub(crate) fn record_trail(&mut self, tick: u64, position: Vec3) {
        if self.last_tick == tick {
            return;
        }
        if tick < self.last_tick {
            self.trail.clear();
        }
        self.last_tick = tick;
        self.trail.push_back(position);
        if self.trail.len() > 1800 {
            self.trail.pop_front();
        }
    }

    pub(crate) fn draw_trail(&self, lines: &mut Builder) {
        if self.show_trail {
            for (a, b) in self.trail.iter().zip(self.trail.iter().skip(1)) {
                if a.distance_squared(*b) < 4000.0f32.powi(2) {
                    lines.line(*a - Vec3::Y * 20.0, *b - Vec3::Y * 20.0, [1.0, 0.85, 0.2]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_is_bounded_and_discards_the_future_after_rewinding() {
        let mut debug = Debug::default();
        for tick in 1..=3600 {
            let position = Vec3::X * tick as f32;
            debug.record_trail(tick, position);
            debug.record_trail(tick, position + Vec3::Z);
        }
        assert_eq!(debug.trail.len(), 1800);
        assert_eq!(debug.trail.front(), Some(&(Vec3::X * 1801.0)));
        assert_eq!(debug.trail.back(), Some(&(Vec3::X * 3600.0)));
        debug.record_trail(30, Vec3::Z);
        assert_eq!(debug.trail.iter().copied().collect::<Vec<_>>(), [Vec3::Z]);
        debug.clear_trail();
        assert!(debug.trail.is_empty());
        debug.record_trail(30, Vec3::X);
        assert_eq!(debug.trail.back(), Some(&Vec3::X));
    }
}
