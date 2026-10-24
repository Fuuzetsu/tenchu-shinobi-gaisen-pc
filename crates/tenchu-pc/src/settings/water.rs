use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Water {
    pub enabled: bool,
    pub ripples: u16,
    pub reflections: u16,
    pub flow: u16,
    pub foam: u16,
    pub spray: u16,
    pub detail: u16,
}

impl Default for Water {
    fn default() -> Self {
        Self {
            enabled: false,
            ripples: 60,
            reflections: 50,
            flow: 100,
            foam: 50,
            spray: 60,
            detail: 1,
        }
    }
}

impl Water {
    pub fn validate(&mut self) {
        self.ripples = self.ripples.min(100);
        self.reflections = self.reflections.min(100);
        self.flow = self.flow.min(200);
        self.foam = self.foam.min(100);
        self.spray = self.spray.min(100);
        self.detail = self.detail.min(2);
    }
}
