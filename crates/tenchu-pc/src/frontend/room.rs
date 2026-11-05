use crate::{
    renderer::sun,
    settings::{Graphics, WeatherMode},
    visuals::sky::Daylight,
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
pub(super) struct Lighting {
    direction: [f32; 4],
    color: [f32; 4],
    horizon: [f32; 4],
    zenith: [f32; 4],
    options: [f32; 4],
    precipitation: [f32; 4],
}

impl Lighting {
    pub fn sample(graphics: &Graphics, seconds: f64, native_snow: bool) -> Option<Self> {
        if !graphics.shop_lighting {
            return None;
        }
        let sky = &graphics.sky;
        let daylight = sky
            .enabled
            .then(|| Daylight::at(f64::from(sky.time) + seconds * f64::from(sky.speed), sky));
        let light = sun::Frame::new(&graphics.lighting, daylight);
        let horizon = daylight.map_or(Vec3::from(graphics.post.fog_color()), |day| day.horizon);
        let zenith = daylight.map_or(horizon, |sky| {
            Vec3::new(0.00015, 0.0004, 0.0015).lerp(Vec3::new(0.018, 0.08, 0.23), sky.day)
        });
        let weather = &graphics.weather;
        let amount = f32::from(weather.amount) / 100.0;
        let rain = if weather.mode.rain() { amount } else { 0.0 };
        let snow = if weather.mode == WeatherMode::Original && native_snow {
            0.3
        } else if weather.mode.snow() {
            amount
        } else {
            0.0
        };
        let velocity = weather.direction();
        Some(Self {
            direction: light
                .direction
                .extend(if graphics.lighting.enabled || sky.enabled {
                    light.strength
                } else {
                    0.0
                })
                .to_array(),
            color: light
                .color
                .extend(f32::from(graphics.lighting.shadows))
                .to_array(),
            horizon: horizon
                .extend(rain * f32::from(weather.haze) / 100.0)
                .to_array(),
            zenith: zenith.extend(f32::from(sky.enabled)).to_array(),
            options: [
                light.day,
                if graphics.lighting.enabled || sky.enabled {
                    light.ambient
                } else {
                    1.0
                },
                if graphics.lighting.enabled && graphics.lighting.lamps {
                    f32::from(graphics.lighting.intensity) / 150.0
                } else {
                    0.0
                },
                graphics.lighting.softness as f32,
            ],
            precipitation: [
                rain,
                snow,
                if rain + snow > 0.0 {
                    seconds as f32
                } else {
                    0.0
                },
                if weather.mode == WeatherMode::Original {
                    0.0
                } else {
                    velocity.x / velocity.y
                },
            ],
        })
    }
}
