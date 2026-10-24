use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sky {
    pub enabled: bool,
    pub time: u16,
    pub speed: u16,
    pub sun: u16,
    pub moon: u16,
    pub day_ambient: u16,
    pub night_ambient: u16,
    pub heading: u16,
    pub relighting: u16,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            enabled: false,
            time: 720,
            speed: 0,
            sun: 100,
            moon: 25,
            day_ambient: 65,
            night_ambient: 30,
            heading: 0,
            relighting: 90,
        }
    }
}

impl Sky {
    pub fn validate(&mut self) {
        self.time %= 1440;
        self.speed = self.speed.min(120);
        self.sun = self.sun.min(200);
        self.moon = self.moon.min(200);
        self.day_ambient = self.day_ambient.min(100);
        self.night_ambient = self.night_ambient.min(100);
        self.heading %= 360;
        self.relighting = self.relighting.min(100);
    }
}
