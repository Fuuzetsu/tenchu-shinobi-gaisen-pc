use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Post {
    pub enabled: bool,
    pub bloom: u16,
    pub bloom_threshold: u16,
    pub occlusion: u16,
    pub occlusion_radius: u16,
    pub filmic: bool,
    pub contrast: u16,
    pub saturation: u16,
    pub temperature: i16,
    pub gamma: u16,
    pub vignette: u16,
    pub sharpen: u16,
    pub fog_tint: u16,
    pub haze: u16,
}

impl Default for Post {
    fn default() -> Self {
        Self {
            enabled: false,
            bloom: 15,
            bloom_threshold: 100,
            occlusion: 35,
            occlusion_radius: 8,
            filmic: true,
            contrast: 100,
            saturation: 100,
            temperature: 0,
            gamma: 100,
            vignette: 0,
            sharpen: 0,
            fog_tint: 1,
            haze: 0,
        }
    }
}

impl Post {
    pub fn validate(&mut self) {
        self.bloom = self.bloom.min(100);
        self.bloom_threshold = self.bloom_threshold.clamp(25, 300);
        self.occlusion = self.occlusion.min(100);
        self.occlusion_radius = self.occlusion_radius.clamp(2, 20);
        self.contrast = self.contrast.clamp(50, 150);
        self.saturation = self.saturation.min(200);
        self.temperature = self.temperature.clamp(-50, 50);
        self.gamma = self.gamma.clamp(50, 150);
        self.vignette = self.vignette.min(100);
        self.sharpen = self.sharpen.min(100);
        self.fog_tint = self.fog_tint.min(3);
        self.haze = self.haze.min(100);
    }

    pub fn fog_color(&self) -> [f32; 3] {
        if !self.enabled {
            return [0.0; 3];
        }
        match self.fog_tint {
            1 => [0.003, 0.006, 0.012],
            2 => [0.015, 0.009, 0.006],
            3 => [0.02, 0.025, 0.03],
            _ => [0.0; 3],
        }
    }
}
