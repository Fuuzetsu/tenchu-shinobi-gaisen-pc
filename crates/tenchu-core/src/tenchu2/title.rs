#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Choice {
    Campaign,
    Editor,
    Options,
}

impl Choice {
    pub const ALL: [Self; 3] = [Self::Campaign, Self::Editor, Self::Options];

    pub fn index(self) -> usize {
        match self {
            Self::Campaign => 0,
            Self::Editor => 1,
            Self::Options => 2,
        }
    }
}

pub struct Title {
    pub choice: Choice,
    pub active: bool,
    pub attract: bool,
    pub fade: i16,
    pub color: [u8; 3],
    pub idle: u32,
    pub tick: u32,
    pub finished: bool,
    previous: u16,
}

impl Default for Title {
    fn default() -> Self {
        Self {
            choice: Choice::Campaign,
            active: false,
            attract: false,
            fade: 240,
            color: [0; 3],
            idle: 0,
            tick: 0,
            finished: false,
            previous: 0,
        }
    }
}

impl Title {
    // Japanese MENU.EXE 0x8010fa3c, one nominal 60 Hz update.
    pub fn step(&mut self, mut buttons: u16) -> Option<u8> {
        if self.finished {
            return None;
        }
        let mut sound = None;
        if self.fade == 0 && self.idle > 1800 {
            self.fade = -8;
            self.attract = true;
        }
        if self.previous == 0 && self.fade == 0 {
            if buttons != 0 {
                self.idle = 0;
                if !self.active {
                    self.active = true;
                    sound = Some(2);
                    buttons = u16::MAX;
                }
            }
            let selection = self.choice.index();
            match buttons {
                0x10 if selection > 0 => {
                    self.choice = Choice::ALL[selection - 1];
                    sound = Some(4);
                }
                0x40 if selection + 1 < Choice::ALL.len() => {
                    self.choice = Choice::ALL[selection + 1];
                    sound = Some(4);
                }
                0x2000 => {
                    sound = Some(2);
                    self.fade = -8;
                }
                _ => {}
            }
        }
        if self.fade != 0 {
            self.fade -= 8;
            if self.fade < -240 {
                self.finished = true;
                return sound;
            }
        }
        if (self.color[0] as i8) >= 0 {
            let blue = self.color[2];
            self.color = [blue, blue, blue.wrapping_add(1)];
        }
        self.previous = buttons;
        self.idle = self.idle.wrapping_add(1);
        self.tick = self.tick.wrapping_add(1);
        sound
    }
}
