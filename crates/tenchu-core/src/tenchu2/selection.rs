use super::progress::Progress;
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::selection::Rules;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Start,
    Cancel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub hero: usize,
    pub displayed: usize,
    pub mission: usize,
    pub page: usize,
    pub phase: u32,
    pub yaw: [i16; 3],
    pub fade: i16,
    pub tick: u32,
    pub outcome: Option<Outcome>,
    pub finished: bool,
    pub progress: Progress,
    pub all_unlocked: bool,
    previous: u16,
    history: [u16; 30],
}

impl Selection {
    pub fn new(progress: Progress, all_unlocked: bool) -> Self {
        Self {
            hero: 0,
            displayed: 0,
            mission: 0,
            page: 0,
            phase: 0,
            yaw: [0; 3],
            fade: 240,
            tick: 0,
            outcome: None,
            finished: false,
            progress,
            all_unlocked,
            previous: 0,
            history: [0; 30],
        }
    }

    pub fn available(&self, mission: usize, rules: &Rules) -> bool {
        mission < 11
            && rules.names[self.hero][mission].is_some()
            && (self.all_unlocked || self.progress.records[self.hero][mission].time_tenths != 0)
    }

    pub fn step(&mut self, buttons: u16, rules: &Rules) -> Vec<u8> {
        if self.finished {
            return Vec::new();
        }
        let mut sounds = Vec::new();
        let previous = std::mem::replace(&mut self.previous, buttons);
        if previous != buttons {
            self.history.copy_within(..29, 1);
            self.history[0] = buttons;
            if self.history[..26] == rules.unlock_hero {
                self.progress.tatsumaru_unlocked = true;
                sounds.push(2);
            }
            if self.history[..18] == rules.unlock_missions {
                for record in self.progress.records.iter_mut().flatten() {
                    if record.time_tenths == 0 {
                        record.time_tenths = 59999;
                    }
                }
                sounds.push(2);
            }
        }
        let old = (self.hero, self.mission);
        if previous == 0 && self.fade == 0 {
            match buttons {
                0x80 | 0x20 if self.hero == self.displayed => {
                    self.hero = (self.hero + if buttons == 0x80 { 2 } else { 1 }) % 3;
                    if self.hero == 2 && !self.all_unlocked && !self.progress.tatsumaru_unlocked {
                        self.hero = usize::from(buttons == 0x80);
                    }
                    self.mission = 0;
                    self.page = 0;
                }
                0x10 if self.mission > 0 => {
                    self.mission -= 1;
                    self.page = self.page.min(self.mission);
                }
                0x40 if self.available(self.mission + 1, rules) => {
                    self.mission += 1;
                    self.page = self.page.max(self.mission.saturating_sub(3));
                }
                0x4000 => {
                    self.outcome = Some(Outcome::Cancel);
                    self.fade = -8;
                }
                0x2000 if self.available(self.mission, rules) => {
                    self.outcome = Some(Outcome::Start);
                    self.fade = -8;
                }
                _ => {}
            }
            if old.0 != self.hero {
                self.yaw[self.hero] = 2048;
                self.yaw[self.displayed] = 0;
            }
            if old != (self.hero, self.mission) {
                sounds.push(4);
            } else if self.fade != 0 {
                sounds.push(2);
            }
        }
        if self.fade != 0 {
            self.fade -= 8;
            if self.fade < -240 {
                self.finished = true;
                return sounds;
            }
        }
        if self.hero != self.displayed {
            self.phase = self.phase.wrapping_add(128);
            self.yaw[self.hero] = self.yaw[self.hero].wrapping_sub(128);
            self.yaw[self.displayed] = self.yaw[self.displayed].wrapping_sub(128);
            if self.phase & 2047 == 0 {
                self.displayed = self.hero;
            }
        }
        self.tick = self.tick.wrapping_add(1);
        sounds
    }
}
