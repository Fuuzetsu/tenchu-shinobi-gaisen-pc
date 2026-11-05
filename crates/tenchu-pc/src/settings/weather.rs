use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherMode {
    #[default]
    Original,
    Clear,
    Rain,
    Snow,
    Rainstorm,
    Snowstorm,
    Blizzard,
}

impl WeatherMode {
    pub const ALL: [Self; 7] = [
        Self::Original,
        Self::Clear,
        Self::Rain,
        Self::Snow,
        Self::Rainstorm,
        Self::Snowstorm,
        Self::Blizzard,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Snow => "Snow",
            Self::Rainstorm => "Rainstorm",
            Self::Snowstorm => "Snowstorm",
            Self::Blizzard => "Blizzard",
        }
    }

    pub fn precipitates(self) -> bool {
        self.rain() || self.snow()
    }

    pub fn rain(self) -> bool {
        matches!(self, Self::Rain | Self::Rainstorm)
    }

    pub fn snow(self) -> bool {
        matches!(self, Self::Snow | Self::Snowstorm | Self::Blizzard)
    }

    pub fn storm(self) -> bool {
        matches!(self, Self::Rainstorm | Self::Snowstorm)
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
    pub reflections: u16,
    pub puddles: u16,
    pub pooling: u16,
    pub haze: u16,
    pub streaks: u16,
    pub snow: u16,
    pub snow_drift: u16,
    pub runoff: bool,
    pub runoff_amount: u16,
    pub footprints: bool,
    pub baked_snow: bool,
    pub footprint_seconds: u16,
    pub lightning: u16,
    pub lightning_interval: u16,
    pub thunder: u16,
    pub thunder_style: tenchu_audio::ThunderStyle,
    pub rain_volume: u16,
    pub wind_volume: u16,
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
            reflections: 100,
            puddles: 55,
            pooling: 100,
            haze: 25,
            streaks: 100,
            snow: 60,
            snow_drift: 65,
            runoff: true,
            runoff_amount: 70,
            footprints: true,
            baked_snow: false,
            footprint_seconds: 60,
            lightning: 40,
            lightning_interval: 25,
            thunder: 60,
            thunder_style: Default::default(),
            rain_volume: 0,
            wind_volume: 70,
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
        self.reflections = self.reflections.min(300);
        self.puddles = self.puddles.min(100);
        self.pooling = self.pooling.clamp(25, 1000);
        self.haze = self.haze.min(100);
        self.streaks = self.streaks.min(200);
        self.runoff_amount = self.runoff_amount.min(100);
        self.snow = self.snow.min(100);
        self.snow_drift = self.snow_drift.min(100);
        self.footprint_seconds = self.footprint_seconds.clamp(10, 180);
        self.lightning = self.lightning.min(100);
        self.lightning_interval = self.lightning_interval.clamp(8, 120);
        self.thunder = self.thunder.min(100);
        self.rain_volume = self.rain_volume.min(100);
        self.wind_volume = self.wind_volume.min(100);
    }

    pub fn particles(&self) -> usize {
        let count =
            [384, 896, 1792][usize::from(self.quality.min(2))] * usize::from(self.amount) / 100;
        if self.mode.storm() || self.mode == WeatherMode::Blizzard {
            count * 3 / 2
        } else {
            count
        }
    }

    pub fn velocity(&self, speed: f32) -> glam::Vec3 {
        let heading = f32::from(self.heading).to_radians();
        let wind = f32::from(self.wind) * speed / self.fall_speed()
            * if self.mode.snow() { 0.65 } else { 1.0 }
            * if self.mode == WeatherMode::Blizzard {
                2.5
            } else {
                1.0
            };
        glam::Vec3::new(heading.sin() * wind, speed, heading.cos() * wind)
    }

    pub fn direction(&self) -> glam::Vec3 {
        self.velocity(self.fall_speed()).normalize()
    }

    fn fall_speed(&self) -> f32 {
        if self.mode.snow() { 36.0 } else { 380.0 }
    }
}
