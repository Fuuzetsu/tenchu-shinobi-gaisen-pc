use tenchu_assets::frontend::CaptionCommand;

#[derive(Default)]
pub struct Captions {
    pub text: Vec<u8>,
    commands: Vec<CaptionCommand>,
    cursor: usize,
}

impl Captions {
    pub fn new(commands: Vec<CaptionCommand>) -> Self {
        Self {
            commands,
            ..Self::default()
        }
    }

    pub fn frame(&mut self, frame: u32) -> Vec<i16> {
        let mut voices = Vec::new();
        while let Some(command) = self.commands.get(self.cursor) {
            match command {
                CaptionCommand::Wait(until) => {
                    if *until != 0 && frame >= *until {
                        self.cursor += 1;
                    }
                    break;
                }
                CaptionCommand::Voice(id) => voices.push(*id),
                CaptionCommand::Text(text) => self.text.clone_from(text),
                CaptionCommand::Clear => self.text.clear(),
            }
            self.cursor += 1;
        }
        voices
    }
}

pub struct Title {
    pub clock: u32,
    pub frames: [i32; 7],
    pub seal_angle: i32,
    pub seal_speed: i32,
    pub fade: i32,
    pub selection: Option<usize>,
    pub finished: bool,
    phase: i16,
}

impl Default for Title {
    fn default() -> Self {
        Self {
            clock: 0,
            frames: [-1; 7],
            seal_angle: 0,
            seal_speed: 50,
            fade: 128,
            selection: None,
            finished: false,
            phase: 0,
        }
    }
}

impl Title {
    // Logo, genre, seals, subtitle, kunai, copyright, blood: TITLE.C's counters.
    pub fn tick(&mut self) -> Vec<i16> {
        self.clock += 1;
        let mut sounds = Vec::new();
        if self.frames[4] == 0 {
            sounds.push(31);
        }
        if self.frames[6] == 0 {
            sounds.push(1);
        }
        for (frame, limit) in self.frames.iter_mut().zip([128, 96, 200, 30, 10, 128, 28]) {
            if *frame >= 0 && *frame < limit {
                *frame += 1;
            }
        }
        if self.frames[2] >= 0 {
            self.seal_angle = (self.seal_angle + self.seal_speed) & 4095;
            if self.frames[2] >= 200 && self.seal_speed > 5 && self.clock.is_multiple_of(10) {
                self.seal_speed -= 1;
            }
        }
        match self.phase {
            0 => {
                self.fade = (self.fade - 8).max(0);
                if self.fade == 0 {
                    self.phase = 1;
                }
            }
            1 if self.clock >= 120 => {
                self.frames[2] = 0;
                self.phase = 2;
            }
            2 if self.clock >= 250 => {
                self.fade -= 8;
                if self.fade < -127 {
                    self.phase = 3;
                }
            }
            3 if self.clock >= 301 => {
                self.fade = (self.fade + 2).min(0);
                if self.fade == 0 {
                    self.phase = 4;
                }
            }
            4 if self.clock >= 400 => {
                self.frames[0] = 0;
                self.phase = 5;
            }
            5 if self.clock >= 500 => {
                self.frames[3] = 0;
                self.phase = 6;
            }
            6 if self.clock >= 610 => {
                self.frames[4] = 0;
                self.phase = 7;
            }
            7 if self.clock >= 620 => {
                self.frames[6] = 0;
                self.phase = 8;
            }
            8 if self.clock >= 650 => {
                self.frames[1] = 0;
                self.phase = 9;
            }
            9 if self.clock >= 700 => {
                self.frames[5] = 0;
                self.phase = 10;
            }
            10 => self.phase = -1,
            -1 if self.clock > 3000 => {
                self.selection = None;
                self.phase = -2;
            }
            -2 => {
                self.fade = (self.fade + 8).min(128);
                self.finished = self.fade == 128;
            }
            _ => (),
        }
        sounds
    }

    pub fn activate(&mut self) -> bool {
        if self.phase == -2 {
            return false;
        }
        self.clock = 0;
        if self.selection.is_none() {
            self.frames = [128, 96, 200, 30, 10, -1, 28];
            self.phase = -1;
            self.fade = 0;
            self.selection = Some(0);
            false
        } else {
            self.phase = -2;
            true
        }
    }

    pub fn navigate(&mut self, delta: i32) -> bool {
        if self.selection.is_none() {
            self.activate();
            return true;
        }
        if self.phase != -1 {
            return false;
        }
        self.clock = 0;
        let old = self.selection.unwrap();
        self.selection = Some((old as i32 + delta).clamp(0, 3) as usize);
        self.selection != Some(old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captions_advance_after_the_wait_frame_and_never_repeat_voices() {
        let mut captions = Captions::new(vec![
            CaptionCommand::Wait(20),
            CaptionCommand::Voice(7),
            CaptionCommand::Text(b"Hello".to_vec()),
            CaptionCommand::Wait(25),
            CaptionCommand::Clear,
            CaptionCommand::Wait(0),
        ]);
        for frame in 1..=20 {
            assert!(captions.frame(frame).is_empty());
        }
        assert_eq!(captions.frame(21), [7]);
        assert_eq!(captions.text, b"Hello");
        for frame in 22..=25 {
            assert!(captions.frame(frame).is_empty());
            assert_eq!(captions.text, b"Hello");
        }
        assert!(captions.frame(26).is_empty());
        assert!(captions.text.is_empty());
        assert!(captions.frame(10000).is_empty());
    }
    #[test]
    fn title_can_be_skipped_selected_and_return_to_the_attract_movie() {
        let mut title = Title::default();
        for _ in 0..119 {
            title.tick();
        }
        assert_eq!(title.frames[2], -1);
        title.tick();
        assert_eq!(title.frames[2], 0);
        assert!(!title.activate());
        assert_eq!(title.selection, Some(0));
        title.navigate(2);
        assert!(title.activate());
        for _ in 0..15 {
            title.tick();
            assert!(!title.finished);
        }
        title.tick();
        assert!(title.finished);
        assert_eq!(title.selection, Some(2));
        let mut title = Title::default();
        for _ in 0..3017 {
            title.tick();
        }
        assert!(title.finished);
        assert!(title.selection.is_none());
    }
}
