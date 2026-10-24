use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Motion {
    pub hair: bool,
    pub cloth: bool,
    pub equipment: bool,
    pub strength: u16,
    pub response: u16,
    pub wind: u16,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            hair: false,
            cloth: false,
            equipment: false,
            strength: 100,
            response: 50,
            wind: 0,
        }
    }
}

impl Motion {
    pub fn actors(&self) -> bool {
        self.strength != 0 && (self.hair || self.cloth || self.equipment)
    }

    pub fn validate(&mut self) {
        self.strength = self.strength.min(200);
        self.response = self.response.min(100);
        self.wind = self.wind.min(100);
    }
}
