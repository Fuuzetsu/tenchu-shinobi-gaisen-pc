use super::Graphics;

pub const NAMES: [&str; 4] = ["Original", "Enhanced", "Cinematic", "Custom"];

impl Graphics {
    pub fn preset(&self) -> u16 {
        let mut current = self.clone();
        current.asset_pack = true;
        (0..3).find(|&index| current == preset(index)).unwrap_or(3)
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
    graphics
}
