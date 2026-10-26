use super::Graphics;

pub const NAMES: [&str; 12] = [
    "Original",
    "Enhanced",
    "Cinematic",
    "Crisp classic",
    "Soft classic",
    "Lantern night",
    "Moonlit",
    "Surface relief",
    "Modern High",
    "Modern Ultra",
    "Showcase",
    "Custom",
];
pub const MODERN_HIGH: u16 = 8;
pub const MODERN_ULTRA: u16 = 9;
pub const SHOWCASE: u16 = 10;
pub const CUSTOM: u16 = NAMES.len() as u16 - 1;

impl Graphics {
    pub fn preset(&self) -> u16 {
        let mut current = self.clone();
        current.asset_pack = true;
        (0..CUSTOM)
            .find(|&index| {
                let mut candidate = preset(index);
                if matches!(index, MODERN_HIGH | MODERN_ULTRA | SHOWCASE) {
                    candidate.weather.mode = current.weather.mode;
                }
                current == candidate
            })
            .unwrap_or(CUSTOM)
    }

    pub fn cycle_preset(&mut self, direction: i32) -> u16 {
        let current = self.preset();
        let index = if current == CUSTOM {
            if direction < 0 { CUSTOM - 1 } else { 0 }
        } else {
            (i32::from(current) + direction).rem_euclid(i32::from(CUSTOM)) as u16
        };
        self.set_preset(index);
        index
    }

    pub fn set_preset(&mut self, index: u16) {
        let artwork = self.asset_pack;
        let weather = self.weather.mode;
        *self = preset(index);
        self.asset_pack = artwork;
        if matches!(index, MODERN_HIGH | MODERN_ULTRA | SHOWCASE) {
            self.weather.mode = weather;
        }
    }
}

fn preset(index: u16) -> Graphics {
    let mut graphics = Graphics::default();
    if index == 0 {
        return graphics;
    }
    if matches!(index, MODERN_HIGH | MODERN_ULTRA) {
        return modern(index == MODERN_ULTRA);
    }
    if index == SHOWCASE {
        let mut g = modern(true);
        g.surface_strength = 60;
        g.lighting.count = 16;
        g.lighting.intensity = 180;
        g.lighting.ambient = 50;
        g.sky.moon = 65;
        g.sky.night_ambient = 65;
        g.exposure = 135;
        g.motion.strength = 100;
        g.water.reflections = 75;
        g.water.foam = 65;
        g.water.spray = 80;
        g.effects.spark_density = 200;
        g.post.bloom = 18;
        g.post.occlusion = 35;
        return g;
    }
    if matches!(index, 3 | 4) {
        graphics.set_filtering(6);
        graphics.magnification = if index == 3 { 1 } else { 2 };
        graphics.samples = 4;
        graphics.interpolation = true;
        return graphics;
    }
    graphics.set_filtering(if index == 2 { 6 } else { 5 });
    graphics.magnification = if index == 2 { 2 } else { 1 };
    graphics.interpolation = true;
    graphics.samples = 4;
    graphics.smooth_models = true;
    graphics.model_detail = 1;
    graphics.blob_shadows = false;
    graphics.exposure = 150;
    let light = &mut graphics.lighting;
    light.enabled = true;
    light.count = 4;
    light.shadow_count = 4;
    light.radius = 10;
    light.intensity = 220;
    light.ambient = 45;
    light.softness = 2;
    light.sky_tint = 15;
    light.directional.intensity = 70;
    light.directional.elevation = 75;
    light.directional.radius = 25;
    let post = &mut graphics.post;
    post.enabled = true;
    post.gamma = 105;
    post.haze = 10;
    if index == 2 {
        light.resolution = 1024;
        light.softness = 3;
        light.sky_tint = 25;
        post.bloom = 25;
        post.occlusion = 50;
        post.haze = 20;
        post.vignette = 10;
    }
    if index == 7 {
        graphics.surface_detail = 2;
        graphics.magnification = 2;
        graphics.surface_strength = 60;
        light.directional.elevation = 40;
    }
    match index {
        5 => {
            graphics.exposure = 125;
            light.ambient = 25;
            light.intensity = 180;
            light.directional.intensity = 25;
            light.resolution = 1024;
            light.softness = 3;
            post.bloom = 10;
            post.temperature = 8;
            post.haze = 5;
            post.occlusion = 40;
        }
        6 => {
            graphics.exposure = 135;
            light.ambient = 35;
            light.intensity = 180;
            light.sky_tint = 55;
            light.directional.intensity = 100;
            light.resolution = 1024;
            post.bloom = 10;
            post.saturation = 90;
            post.temperature = -8;
            post.haze = 15;
        }
        _ => {}
    }
    graphics
}

fn modern(ultra: bool) -> Graphics {
    let mut g = Graphics {
        fog: false,
        blob_shadows: false,
        interpolation: true,
        smooth_models: true,
        model_detail: 1,
        magnification: 1,
        surface_detail: if ultra { 2 } else { 1 },
        surface_strength: 40,
        samples: if ultra { 4 } else { 2 },
        ..Default::default()
    };
    g.set_filtering(if ultra { 6 } else { 5 });
    g.blood.enabled = true;
    g.blood.marks = if ultra { 768 } else { 384 };
    g.water.enabled = true;
    g.water.detail = if ultra { 2 } else { 1 };
    g.models.characters = if ultra { 2 } else { 1 };
    g.models.foliage = if ultra { 4 } else { 3 };
    g.models.variation = true;
    g.models.grapple = true;
    g.effects.sparks = true;
    g.effects.splashes = true;
    g.effects.scatter = true;
    g.effects.leaves = true;
    g.effects.leaf_density = if ultra { 100 } else { 70 };
    g.motion.hair = true;
    g.motion.cloth = true;
    g.motion.equipment = true;
    g.motion.strength = 80;
    g.motion.wind = 20;
    g.weather.quality = if ultra { 2 } else { 1 };
    g.sky.enabled = true;
    g.sky.time = 1320;
    g.sky.moon = 35;
    g.sky.night_ambient = 50;
    g.exposure = 125;
    g.lighting.enabled = true;
    g.lighting.count = 8;
    g.lighting.shadow_count = if ultra { 8 } else { 6 };
    g.lighting.radius = 18;
    g.lighting.intensity = 120;
    g.lighting.ambient = 50;
    g.lighting.softness = 3;
    g.lighting.resolution = if ultra { 2048 } else { 1024 };
    g.lighting.directional.resolution = if ultra { 4096 } else { 2048 };
    g.lighting.directional.radius = 45;
    g.post.enabled = true;
    g.post.bloom = 12;
    g.post.occlusion = 25;
    g.post.sharpen = 10;
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_presets_keep_the_weather_choice_when_cycling() {
        let mut graphics = Graphics::default();
        graphics.weather.mode = crate::settings::WeatherMode::Snow;
        graphics.set_preset(MODERN_HIGH);
        assert_eq!(graphics.preset(), MODERN_HIGH);
        graphics.cycle_preset(1);
        assert_eq!(graphics.preset(), MODERN_ULTRA);
        assert_eq!(graphics.weather.mode, crate::settings::WeatherMode::Snow);
        graphics.cycle_preset(-1);
        assert_eq!(graphics.preset(), MODERN_HIGH);
    }

    #[test]
    fn presets_cycle_and_identify_without_changing_asset_preferences() {
        let mut graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        for index in 0..CUSTOM {
            assert_eq!(graphics.preset(), index);
            assert!(!graphics.asset_pack);
            graphics.cycle_preset(1);
        }
        assert_eq!(graphics.preset(), 0);
        graphics.cycle_preset(-1);
        assert_eq!(graphics.preset(), CUSTOM - 1);
        graphics.fog_distance = 177;
        assert_eq!(graphics.preset(), CUSTOM);
        graphics.cycle_preset(1);
        assert_eq!(graphics.preset(), 0);
    }
}
