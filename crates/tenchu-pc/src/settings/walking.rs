use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Walking {
    pub enabled: bool,
    pub threshold: u16,
}

impl Default for Walking {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 65,
        }
    }
}

impl Walking {
    pub fn validate(&mut self) {
        self.threshold = self.threshold.clamp(25, 95);
    }
}
