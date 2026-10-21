use super::Graphics;

pub const NAMES: [&str; 8] = [
    "Original",
    "Enhanced",
    "Cinematic",
    "Crisp classic",
    "Soft classic",
    "Lantern night",
    "Moonlit",
    "Custom",
];
pub const CUSTOM: u16 = NAMES.len() as u16 - 1;

impl Graphics {
    pub fn preset(&self) -> u16 {
        let mut current = self.clone();
        current.asset_pack = true;
        (0..CUSTOM)
            .find(|&index| current == preset(index))
            .unwrap_or(CUSTOM)
    }

    pub fn cycle_preset(&mut self, direction: i32) {
        let current = self.preset();
        let index = if current == CUSTOM {
            if direction < 0 { CUSTOM - 1 } else { 0 }
        } else {
            (i32::from(current) + direction).rem_euclid(i32::from(CUSTOM)) as u16
        };
        self.set_preset(index);
    }

    pub fn set_preset(&mut self, index: u16) {
        let artwork = self.asset_pack;
        *self = preset(index);
        self.asset_pack = artwork;
    }
}

fn preset(index: u16) -> Graphics {
    let mut graphics = Graphics::default();
    if index == 0 {
        return graphics;
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

#[cfg(test)]
mod tests {
    use super::*;

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
