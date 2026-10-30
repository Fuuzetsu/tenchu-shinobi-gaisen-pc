use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Blood {
    pub enabled: bool,
    pub spray: bool,
    pub amount: u16,
    pub seconds: u16,
    pub marks: u16,
}

impl Default for Blood {
    fn default() -> Self {
        Self {
            enabled: false,
            spray: false,
            amount: 100,
            seconds: 12,
            marks: 256,
        }
    }
}

impl Blood {
    pub fn validate(&mut self) {
        self.amount = self.amount.min(200);
        self.seconds = self.seconds.min(60);
        self.marks = self.marks.clamp(32, 1024);
    }
}
