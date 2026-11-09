use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Title,
    Narration,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Continue,
    Quit,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    pub title: bool,
    pub fade: i16,
    pub columns: Vec<(usize, i16, u8)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Introduction {
    pub phase: Phase,
    pub outcome: Outcome,
    pub frame: Frame,
    pub tick: u32,
    title: i16,
    fade: i16,
    columns: [i16; 16],
    previous: u16,
    narration: bool,
    skipped: bool,
}

impl Introduction {
    pub fn new(narration: bool) -> Self {
        Self {
            phase: Phase::Title,
            outcome: Outcome::Continue,
            frame: Frame {
                title: true,
                fade: 256,
                columns: Vec::new(),
            },
            tick: 0,
            title: 256,
            fade: 256,
            columns: [0; 16],
            previous: 0,
            narration,
            skipped: false,
        }
    }

    pub fn vblanks(&self) -> u8 {
        if self.phase == Phase::Narration { 4 } else { 1 }
    }

    pub fn step(&mut self, buttons: u16, audio_finished: bool) {
        let quit = buttons == 9;
        let skip = self.previous == 0 && matches!(buttons, 8 | 0x2000);
        self.previous = buttons;
        self.frame.columns.clear();
        match self.phase {
            Phase::Title => {
                if self.title > 0 && (quit || skip) {
                    self.outcome = if quit {
                        Outcome::Quit
                    } else {
                        Outcome::Continue
                    };
                    self.skipped = true;
                    self.title = -self.title;
                }
                self.frame.title = true;
                self.frame.fade = self.title.abs();
                self.title -= 1;
                if self.title <= -256 {
                    self.phase = if self.skipped || !self.narration {
                        Phase::Finished
                    } else {
                        Phase::Narration
                    };
                }
            }
            Phase::Narration => {
                self.frame.title = false;
                if audio_finished || quit || skip {
                    self.outcome = if quit {
                        Outcome::Quit
                    } else {
                        Outcome::Continue
                    };
                    if self.fade >= 0 {
                        self.fade -= 256;
                    }
                }
                for (index, age) in self.columns.iter_mut().enumerate() {
                    if self.tick == (index as u32 + 1) * 40 {
                        *age = 1;
                    } else if *age != 0 {
                        let x = *age - 160;
                        let intensity = (128 - (x.abs() - 128).max(0) * 4) as u8;
                        self.frame.columns.push((index, x, intensity));
                        *age += 1;
                        if *age > 320 {
                            *age = 0;
                        }
                    }
                }
                if self.fade > 0 {
                    self.fade -= 4;
                    self.frame.fade = self.fade;
                } else if self.fade < 0 {
                    self.fade += 4;
                    self.frame.fade = self.fade + 256;
                    if self.fade >= 0 {
                        self.phase = Phase::Finished;
                    }
                } else {
                    self.frame.fade = 0;
                }
                self.tick += 1;
            }
            Phase::Finished => {}
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prologue {
    pub frame: Frame,
    pub tick: u32,
    pub finished: bool,
    fade: i16,
    columns: [i16; 20],
    previous: u16,
}

impl Default for Prologue {
    fn default() -> Self {
        Self {
            frame: Frame {
                fade: 128,
                ..Default::default()
            },
            tick: 0,
            finished: false,
            fade: 128,
            columns: [i16::MIN; 20],
            previous: u16::MAX,
        }
    }
}

impl Prologue {
    pub fn step(&mut self, buttons: u16, audio_finished: bool) {
        if self.finished {
            return;
        }
        let mut fade = 0;
        if self.fade == 0 {
            if audio_finished || self.previous == 0 && matches!(buttons, 8 | 0x2000) {
                self.fade = -8;
            }
        } else {
            self.fade -= 8;
            if self.fade < -240 {
                self.finished = true;
                return;
            }
            fade = self.fade.abs();
        }
        self.frame.fade = fade;
        if self.tick.is_multiple_of(30) && self.tick / 30 < 20 {
            self.columns[(self.tick / 30) as usize] = -160;
        }
        self.frame.columns.clear();
        for (index, x) in self.columns.iter_mut().enumerate() {
            if *x != i16::MIN {
                let intensity = (128 - (x.abs() - 128).max(0) * 4) as u8;
                self.frame.columns.push((index, *x, intensity));
                *x += 1;
                if *x >= 160 {
                    *x = i16::MIN;
                }
            }
        }
        self.previous = buttons;
        self.tick += 1;
    }
}
