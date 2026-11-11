use crate::effects::RumbleAction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Feedback {
    pub motors: [u8; 2],
    pub remaining: i16,
    pub action: Option<RumbleAction>,
}

impl Feedback {
    pub fn pulse(&mut self, action: RumbleAction, level: i16, duration: i16) {
        let power = level.wrapping_mul(63);
        self.motors = [u8::from(power > 127), power as u8];
        self.remaining = duration.wrapping_mul(5);
        self.action = Some(action);
    }

    pub fn drowning(&mut self, power: i16) {
        self.motors = [1, power as u8];
        self.remaining = 5;
        self.action = Some(RumbleAction::Drowning);
    }

    pub fn poll(&mut self) {
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.motors = [0; 2];
            }
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
