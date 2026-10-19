use crate::TICKS_PER_SECOND;
use std::time::Duration;

const SECOND: u128 = 1_000_000_000;

#[derive(Default)]
pub struct GameClock {
    phase: u128,
    ticks: u64,
}

impl GameClock {
    pub fn elapse(&mut self, elapsed: Duration) {
        self.phase = self
            .phase
            .saturating_add(elapsed.as_nanos() * u128::from(TICKS_PER_SECOND));
    }

    pub fn step(&mut self) -> Option<u64> {
        if self.phase < SECOND {
            return None;
        }
        self.phase -= SECOND;
        let tick = self.ticks;
        self.ticks += 1;
        Some(tick)
    }

    pub fn interpolation(&self) -> f32 {
        (self.phase.min(SECOND) as f64 / SECOND as f64) as f32
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn discard_host_backlog(&mut self) {
        self.phase = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(frame_times: impl IntoIterator<Item = u64>) -> (u64, i32) {
        let mut clock = GameClock::default();
        let mut position = 0i32;
        for elapsed in frame_times {
            clock.elapse(Duration::from_nanos(elapsed));
            while let Some(tick) = clock.step() {
                position = position.wrapping_add(if tick % 17 < 9 { 71 } else { -53 });
            }
        }
        (clock.ticks(), position)
    }

    #[test]
    fn display_cadence_and_stalls_do_not_change_tick_sequence() {
        let expected = run([10_000_000_000]);
        assert_eq!(expected.0, 300);
        for fps in [24, 30, 60, 75, 144, 240] {
            let frames =
                (0..fps * 10).map(|i| (i + 1) * 1_000_000_000 / fps - i * 1_000_000_000 / fps);
            assert_eq!(run(frames), expected);
        }
        assert_eq!(
            run([1, 999_999_999, 4_000_000_000, 5_000_000_000]),
            expected
        );
    }

    #[test]
    fn pause_discards_wall_time_without_advancing_game_state() {
        let mut clock = GameClock::default();
        clock.elapse(Duration::from_secs(5));
        assert_eq!(clock.step(), Some(0));
        clock.discard_host_backlog();
        assert_eq!(clock.step(), None);
        clock.elapse(Duration::from_nanos(33_333_334));
        assert_eq!(clock.step(), Some(1));
    }
}
