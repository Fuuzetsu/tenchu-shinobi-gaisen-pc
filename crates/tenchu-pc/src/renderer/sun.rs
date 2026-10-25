use crate::{
    settings::{Directional, Lighting},
    visuals::sky::Daylight,
};
use glam::{Mat4, Vec3};

pub fn direction(settings: &Directional) -> Vec3 {
    let azimuth = f32::from(settings.azimuth).to_radians();
    let elevation = f32::from(settings.elevation).to_radians();
    Vec3::new(
        azimuth.sin() * elevation.cos(),
        -elevation.sin(),
        azimuth.cos() * elevation.cos(),
    )
}

pub fn color(settings: &Directional) -> [f32; 3] {
    match settings.color {
        1 => [0.65, 0.78, 1.0],
        2 => [1.0, 0.85, 0.65],
        _ => [1.0; 3],
    }
}

pub fn matrix(settings: &Directional, camera: Mat4, direction: Vec3) -> Mat4 {
    let inverse = camera.inverse();
    let near = inverse.project_point3(Vec3::ZERO);
    let far = inverse.project_point3(Vec3::Z);
    let focus = near + (far - near).normalize() * 8000.0;
    let view = Mat4::look_to_rh(Vec3::ZERO, -direction, Vec3::NEG_Y);
    let mut center = view.transform_point3(focus);
    let radius = f32::from(settings.radius) * 1000.0;
    let texel = 2.0 * radius / settings.resolution as f32;
    center.x = (center.x / texel).floor() * texel;
    center.y = (center.y / texel).floor() * texel;
    Mat4::orthographic_rh(
        center.x - radius,
        center.x + radius,
        center.y - radius,
        center.y + radius,
        -center.z - 100_000.0,
        -center.z + 100_000.0,
    ) * view
}

pub fn target(device: &wgpu::Device, resolution: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Directional shadow"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

pub struct Frame {
    pub settings: Lighting,
    pub direction: Vec3,
    pub color: Vec3,
    pub strength: f32,
    pub ambient: f32,
    pub tint: f32,
    pub weather: crate::visuals::weather::SurfaceWeather,
    pub rain: [f32; 4],
    pub weather_resolution: u32,
    pub precipitation: Vec3,
    pub visual_time: f32,
    pub wind: f32,
}

impl Frame {
    pub fn new(settings: &Lighting, daylight: Option<Daylight>) -> Self {
        let mut frame = Self {
            settings: settings.clone(),
            direction: direction(&settings.directional),
            color: Vec3::from(color(&settings.directional)),
            strength: f32::from(settings.directional.intensity) / 400.0,
            ambient: f32::from(settings.ambient) / 100.0,
            tint: f32::from(settings.sky_tint) / 100.0,
            weather: Default::default(),
            rain: [0.0; 4],
            weather_resolution: 1,
            precipitation: Vec3::Y,
            visual_time: 0.0,
            wind: 0.0,
        };
        if let Some(sky) = daylight {
            frame.settings.enabled = true;
            frame.settings.directional.intensity = 1;
            frame.direction = sky.direction;
            frame.color = sky.color;
            frame.strength = sky.strength;
            frame.ambient = sky.ambient;
            frame.tint = sky.tint;
        }
        frame
    }

    pub fn with_motion(mut self, time: f32, settings: &crate::settings::Motion) -> Self {
        self.visual_time = time;
        self.wind = f32::from(settings.wind) / 100.0;
        self
    }

    pub fn with_weather(
        mut self,
        weather: crate::visuals::weather::SurfaceWeather,
        settings: &crate::settings::Weather,
    ) -> Self {
        if settings.mode != crate::settings::WeatherMode::Original {
            self.weather = weather;
            let amount = if settings.mode == crate::settings::WeatherMode::Rain {
                f32::from(settings.amount) / 100.0
            } else {
                0.0
            };
            self.rain = [
                f32::from(settings.reflections) / 100.0,
                amount * f32::from(settings.haze) / 100.0,
                amount,
                f32::from(settings.snow_drift) / 100.0,
            ];
            self.precipitation = settings.direction();
            if weather.wet > 0.0
                || weather.snow > 0.0
                || weather.puddles > 0.0
                || self.rain[1] > 0.0
            {
                self.weather_resolution = [256, 512, 1024][usize::from(settings.quality.min(2))];
            }
        }
        self
    }
}
