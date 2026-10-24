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
    pub distance: u16,
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
            distance: 160,
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
        self.distance = self.distance.clamp(20, 240);
        self.wetness = self.wetness.min(100);
        self.snow = self.snow.min(100);
    }

    pub fn particles(&self) -> usize {
        [384, 896, 1792][usize::from(self.quality.min(2))] * usize::from(self.amount) / 100
    }

    pub fn velocity(&self, speed: f32) -> glam::Vec3 {
        let heading = f32::from(self.heading).to_radians();
        let wind = f32::from(self.wind)
            * if self.mode == WeatherMode::Snow {
                0.65
            } else {
                1.0
            };
        glam::Vec3::new(heading.sin() * wind, speed, heading.cos() * wind)
    }

    pub fn direction(&self) -> glam::Vec3 {
        self.velocity(if self.mode == WeatherMode::Snow {
            36.0
        } else {
            380.0
        })
        .normalize()
    }
}
