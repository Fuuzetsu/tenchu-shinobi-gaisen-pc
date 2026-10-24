use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherMode {
    #[default]
    Original,
    Clear,
    Rain,
    Snow,
}

impl WeatherMode {
    pub const ALL: [Self; 4] = [Self::Original, Self::Clear, Self::Rain, Self::Snow];

    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Snow => "Snow",
        }
    }

    pub fn precipitates(self) -> bool {
        matches!(self, Self::Rain | Self::Snow)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weather {
    pub mode: WeatherMode,
    pub amount: u16,
    pub wind: u16,
    pub heading: u16,
    pub quality: u16,
    pub wetness: u16,
    pub snow: u16,
    pub runoff: bool,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            mode: WeatherMode::Original,
            amount: 70,
            wind: 25,
            heading: 45,
            quality: 1,
            wetness: 70,
            snow: 60,
            runoff: true,
        }
    }
}

impl Weather {
    pub fn validate(&mut self) {
        self.amount = self.amount.min(100);
        self.wind = self.wind.min(100);
        self.heading %= 360;
        self.quality = self.quality.min(2);
        self.wetness = self.wetness.min(100);
        self.snow = self.snow.min(100);
    }

    pub fn particles(&self) -> usize {
        [384, 896, 1792][usize::from(self.quality.min(2))] * usize::from(self.amount) / 100
    }
}
