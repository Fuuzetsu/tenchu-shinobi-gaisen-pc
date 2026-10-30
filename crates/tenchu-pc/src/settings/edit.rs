use super::{Settings, menu::Item};

pub struct Slider {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub scale: i32,
}

impl Item {
    pub fn slider(self) -> Option<Slider> {
        let (min, max, step, scale) = match self {
            Item::VibrationIntensity | Item::VibrationAction(_) => (0, 200, 5, 1),
            Item::MotionStrength => (0, 200, 5, 1),
            Item::MotionResponse | Item::MotionWind => (0, 100, 5, 1),
            Item::WeatherAmount
            | Item::WeatherWind
            | Item::Wetness
            | Item::SnowCover
            | Item::SnowDrift
            | Item::PuddleCoverage
            | Item::RainHaze
            | Item::RunoffAmount => (0, 100, 5, 1),
            Item::RainReflections => (0, 300, 10, 1),
            Item::RainStreaks => (0, 200, 10, 1),
            Item::PoolingSpeed => (25, 1000, 25, 1),
            Item::WeatherHeading => (0, 345, 15, 1),
            Item::Inventory(_) => (0, 255, 1, 1),
            Item::FootprintSeconds => (10, 180, 10, 1),
            Item::WeatherDistance => (20, 240, 20, 1),
            Item::WaterRipples | Item::WaterReflections | Item::WaterFoam | Item::WaterSpray => {
                (0, 100, 5, 1)
            }
            Item::WaterFlow => (0, 200, 10, 1),
            Item::TimeOfDay => (0, 1425, 15, 1),
            Item::TimeSpeed => (0, 120, 1, 1),
            Item::SunPath => (0, 345, 15, 1),
            Item::SunStrength | Item::MoonStrength => (0, 200, 5, 1),
            Item::DayAmbient | Item::NightAmbient | Item::DayRelighting => (0, 100, 5, 1),
            Item::BloodAmount => (0, 200, 10, 1),
            Item::BloodSeconds => (0, 60, 1, 1),
            Item::BloodMarks => (32, i32::from(u16::MAX), 256, 1),
            Item::Volume | Item::SurfaceStrength => (0, 100, 5, 1),
            Item::FogDistance | Item::SightDistance => {
                (if self == Item::FogDistance { 25 } else { 0 }, 400, 25, 1)
            }
            Item::Exposure => (25, 300, 25, 1),
            Item::FlightSpeed => (2, 100, 2, 1),
            Item::Fov => (20, 110, 5, 1),
            Item::LightRadius => (2, 50, 1, 1),
            Item::LightIntensity => (0, 400, 10, 1),
            Item::LightAmbient => (0, 100, 5, 1),
            Item::SparkDensity => (0, 300, 10, 1),
            Item::LeafDensity => (0, 200, 10, 1),
            Item::ShadowSoftness => (0, 8, 1, 1),
            Item::LightSourceSize => (0, 400, 10, 1),
            Item::SkyTint => (0, 100, 10, 1),
            Item::DirectionalIntensity => (0, 200, 10, 1),
            Item::DirectionalAzimuth => (0, 345, 15, 1),
            Item::DirectionalElevation => (10, 85, 5, 1),
            Item::DirectionalRadius => (10, 60, 5, 1),
            Item::Bloom | Item::Occlusion | Item::Vignette | Item::Sharpen | Item::Haze => {
                (0, 100, 5, 1)
            }
            Item::BloomThreshold => (25, 300, 25, 1),
            Item::OcclusionRadius => (2, 20, 1, 100),
            Item::Contrast | Item::Gamma => (50, 150, 5, 1),
            Item::Saturation => (0, 200, 5, 1),
            Item::Temperature => (-50, 50, 5, 1),
            _ => return None,
        };
        Some(Slider {
            min,
            max,
            step,
            scale,
        })
    }

    fn boolean(self, settings: &mut Settings) -> Option<&mut bool> {
        Some(match self {
            Self::ModelMoveset => &mut settings.model_moveset,
            Self::ModelPlayerMovement => &mut settings.model_moveset_options.player_movement,
            Self::ModelNoIdleTaunts => &mut settings.model_moveset_options.no_idle_taunts,
            Self::Vibration => &mut settings.frontend.rumble,
            Self::GrappleDetail => &mut settings.graphics.models.grapple,
            Self::ContactSparks => &mut settings.graphics.effects.sparks,
            Self::Lightning => &mut settings.graphics.effects.lightning,
            Self::Gunfire => &mut settings.graphics.effects.gunfire,
            Self::ContactSplashes => &mut settings.graphics.effects.splashes,
            Self::ContactScatter => &mut settings.graphics.effects.scatter,
            Self::FallingLeaves => &mut settings.graphics.effects.leaves,
            Self::HairMotion => &mut settings.graphics.motion.hair,
            Self::ClothMotion => &mut settings.graphics.motion.cloth,
            Self::EquipmentMotion => &mut settings.graphics.motion.equipment,
            Self::VegetationVariation => &mut settings.graphics.models.variation,
            Self::Water => &mut settings.graphics.water.enabled,
            Self::Footprints => &mut settings.graphics.weather.footprints,
            Self::RoofRunoff => &mut settings.graphics.weather.runoff,
            Self::TimeClock => &mut settings.graphics.sky.enabled,
            Self::Blood => &mut settings.graphics.blood.enabled,
            Self::BloodSpray => &mut settings.graphics.blood.spray,
            Self::Interpolation => &mut settings.graphics.interpolation,
            Self::Retro => &mut settings.retro,
            Self::Post => &mut settings.graphics.post.enabled,
            Self::Filmic => &mut settings.graphics.post.filmic,
            Self::SmoothModels => &mut settings.graphics.smooth_models,
            Self::AssetPack => &mut settings.graphics.asset_pack,
            Self::SharpText => &mut settings.sharp_text,
            Self::Lighting => &mut settings.graphics.lighting.enabled,
            Self::Shadows => &mut settings.graphics.lighting.shadows,
            Self::FireLights => &mut settings.graphics.lighting.fires,
            Self::LampLights => &mut settings.graphics.lighting.lamps,
            Self::LightMarkers => &mut settings.graphics.lighting.markers,
            Self::BlobShadows => &mut settings.graphics.blob_shadows,
            Self::Performance => &mut settings.show_performance,
            Self::Fps => &mut settings.show_fps,
            Self::Focus => &mut settings.pause_on_defocus,
            Self::MenuPause => &mut settings.pause_in_menu,
            Self::Fullscreen => &mut settings.fullscreen,
            Self::Vsync => &mut settings.vsync,
            Self::Hud => &mut settings.show_hud,
            Self::Fog => &mut settings.graphics.fog,
            Self::Textures => &mut settings.graphics.textures,
            Self::Shading => &mut settings.graphics.shading,
            _ => return None,
        })
    }

    fn number(self, settings: &mut Settings) -> Option<&mut u16> {
        Some(match self {
            Self::VibrationIntensity => &mut settings.vibration.intensity,
            Self::VibrationAction(action) => {
                settings.vibration.actions.entry(action).or_insert(100)
            }
            Self::SparkDensity => &mut settings.graphics.effects.spark_density,
            Self::LeafDensity => &mut settings.graphics.effects.leaf_density,
            Self::MotionStrength => &mut settings.graphics.motion.strength,
            Self::MotionResponse => &mut settings.graphics.motion.response,
            Self::MotionWind => &mut settings.graphics.motion.wind,
            Self::WaterRipples => &mut settings.graphics.water.ripples,
            Self::WaterReflections => &mut settings.graphics.water.reflections,
            Self::WaterFlow => &mut settings.graphics.water.flow,
            Self::WaterFoam => &mut settings.graphics.water.foam,
            Self::WaterSpray => &mut settings.graphics.water.spray,
            Self::WeatherAmount => &mut settings.graphics.weather.amount,
            Self::WeatherWind => &mut settings.graphics.weather.wind,
            Self::WeatherHeading => &mut settings.graphics.weather.heading,
            Self::WeatherDistance => &mut settings.graphics.weather.distance,
            Self::Wetness => &mut settings.graphics.weather.wetness,
            Self::RainReflections => &mut settings.graphics.weather.reflections,
            Self::PuddleCoverage => &mut settings.graphics.weather.puddles,
            Self::PoolingSpeed => &mut settings.graphics.weather.pooling,
            Self::RainHaze => &mut settings.graphics.weather.haze,
            Self::RainStreaks => &mut settings.graphics.weather.streaks,
            Self::RunoffAmount => &mut settings.graphics.weather.runoff_amount,
            Self::FootprintSeconds => &mut settings.graphics.weather.footprint_seconds,
            Self::SnowCover => &mut settings.graphics.weather.snow,
            Self::SnowDrift => &mut settings.graphics.weather.snow_drift,
            Self::TimeOfDay => &mut settings.graphics.sky.time,
            Self::TimeSpeed => &mut settings.graphics.sky.speed,
            Self::SunStrength => &mut settings.graphics.sky.sun,
            Self::MoonStrength => &mut settings.graphics.sky.moon,
            Self::DayAmbient => &mut settings.graphics.sky.day_ambient,
            Self::NightAmbient => &mut settings.graphics.sky.night_ambient,
            Self::SunPath => &mut settings.graphics.sky.heading,
            Self::DayRelighting => &mut settings.graphics.sky.relighting,
            Self::BloodAmount => &mut settings.graphics.blood.amount,
            Self::BloodSeconds => &mut settings.graphics.blood.seconds,
            Self::BloodMarks => &mut settings.graphics.blood.marks,
            Self::Bloom => &mut settings.graphics.post.bloom,
            Self::BloomThreshold => &mut settings.graphics.post.bloom_threshold,
            Self::Occlusion => &mut settings.graphics.post.occlusion,
            Self::OcclusionRadius => &mut settings.graphics.post.occlusion_radius,
            Self::Contrast => &mut settings.graphics.post.contrast,
            Self::Saturation => &mut settings.graphics.post.saturation,
            Self::Gamma => &mut settings.graphics.post.gamma,
            Self::Vignette => &mut settings.graphics.post.vignette,
            Self::Sharpen => &mut settings.graphics.post.sharpen,
            Self::Haze => &mut settings.graphics.post.haze,
            Self::SurfaceStrength => &mut settings.graphics.surface_strength,
            Self::Volume => &mut settings.volume,
            Self::FogDistance => &mut settings.graphics.fog_distance,
            Self::Exposure => &mut settings.graphics.exposure,
            Self::SkyTint => &mut settings.graphics.lighting.sky_tint,
            Self::DirectionalIntensity => &mut settings.graphics.lighting.directional.intensity,
            Self::DirectionalAzimuth => &mut settings.graphics.lighting.directional.azimuth,
            Self::DirectionalElevation => &mut settings.graphics.lighting.directional.elevation,
            Self::DirectionalRadius => &mut settings.graphics.lighting.directional.radius,
            Self::LightSourceSize => &mut settings.graphics.lighting.source_size,
            Self::ShadowSoftness => &mut settings.graphics.lighting.softness,
            Self::LightRadius => &mut settings.graphics.lighting.radius,
            Self::LightIntensity => &mut settings.graphics.lighting.intensity,
            Self::LightAmbient => &mut settings.graphics.lighting.ambient,
            _ => return None,
        })
    }

    pub fn change_setting(self, settings: &mut Settings, direction: i32) -> bool {
        if let Some(value) = self.boolean(settings) {
            *value = !*value;
        } else if let Some(value) = self.number(settings) {
            let slider = self.slider().expect("numeric preference has a slider");
            let next = i32::from(*value) + direction * slider.step;
            *value = match self {
                Self::TimeOfDay => next.rem_euclid(1440),
                Self::WeatherHeading | Self::SunPath | Self::DirectionalAzimuth => {
                    next.rem_euclid(360)
                }
                _ => next.clamp(slider.min, slider.max),
            } as u16;
        } else {
            return false;
        }
        true
    }

    pub fn reset_setting(self, settings: &mut Settings) -> bool {
        let mut defaults = Settings::default();
        if let Some(value) = self.boolean(settings) {
            *value = *self.boolean(&mut defaults).unwrap();
        } else if let Some(value) = self.number(settings) {
            *value = *self.number(&mut defaults).unwrap();
        } else {
            return false;
        }
        true
    }
}
