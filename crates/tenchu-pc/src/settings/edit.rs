use super::{Settings, menu::Item};

pub enum Preference {
    CheckForUpdates(bool),
    Touch(crate::touch::Settings),
    PlayerModel(String),
    MovesetModel(String),
    WeaponModel(String),
    VoiceModel(String),
}

impl Preference {
    pub fn apply(self, settings: &mut Settings) {
        match self {
            Self::CheckForUpdates(enabled) => settings.check_for_updates = enabled,
            Self::Touch(value) => settings.touch = value,
            Self::PlayerModel(id) => settings.player_model = id,
            Self::MovesetModel(id) => {
                settings.moveset_model = id;
                settings.model_moveset = true;
            }
            Self::WeaponModel(id) => settings.weapon_model = id,
            Self::VoiceModel(id) => settings.voice_model = id,
        }
    }

    pub fn capability(&self) -> Option<Item> {
        match self {
            Self::CheckForUpdates(_) | Self::Touch(_) => None,
            Self::PlayerModel(_)
            | Self::MovesetModel(_)
            | Self::WeaponModel(_)
            | Self::VoiceModel(_) => Some(Item::PlayerModel),
        }
    }
}

pub struct Slider {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub scale: i32,
}

impl Item {
    pub fn slider(self) -> Option<Slider> {
        let (min, max, step, scale) = match self {
            Self::BurningVolume => (0, 100, 5, 1),
            Self::BurningRange => (500, 30000, 500, 1),
            Self::BleedChance => (0, 100, 5, 1),
            Self::SeverStealth
            | Self::SeverDive
            | Self::SeverArms
            | Self::SeverSide
            | Self::PlayerSeverHead
            | Self::PlayerSeverArms
            | Self::PlayerSeverSide => (0, 100, 1, 1),
            Self::BleedSeconds => (0, 120, 1, 1),
            Self::BleedDamage => (0, 20, 1, 1),
            Self::BandageLoadout | Self::Bandages => (0, 99, 1, 1),
            Item::BehaviorSeed => (0, 65535, 1, 1),
            Item::RewindHistory => (0, 30, 5, 1),
            Item::RewindSpeed => (25, 800, 25, 1),
            Item::AlternateDeathChance => (0, 100, 5, 1),
            Item::MouseSensitivity | Item::StickSensitivity => (10, 300, 5, 1),
            Item::WalkThreshold => (25, 95, 5, 1),
            Item::SheathedSpeed => (25, 200, 5, 1),
            Item::SheathedVisibility => (25, 100, 5, 1),
            Item::SheathedDamage => (100, 400, 25, 1),
            Item::StickDeadzone => (5, 40, 1, 1),
            Item::FollowDistance => (20, 100, 2, 1),
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
            Item::StormLightning | Item::StormThunder | Item::RainVolume | Item::WindVolume => {
                (0, 100, 5, 1)
            }
            Item::StormInterval => (8, 120, 1, 1),
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
            Item::BloodAmount
            | Item::BloodSpurts
            | Item::DeathGush
            | Item::WeaponBloodCoverage
            | Item::WeaponBloodFlow
            | Item::WeaponReflections => (0, 200, 10, 1),
            Item::SeveredBlood => (0, 200, 10, 1),
            Item::SeveredSeconds => (0, 15, 1, 1),
            Item::BloodDeathSize => (10, 200, 10, 1),
            Item::BloodSeconds => (0, 60, 1, 1),
            Item::GushSeconds => (0, 15, 1, 1),
            Item::BloodMarks => (32, i32::from(u16::MAX), 256, 1),
            Item::ReinforcementLimit => (0, 1000, 10, 1),
            Item::DetectionDamage => (0, 100, 10, 1),
            Item::DetectionDamageLimit => (0, 400, 25, 1),
            Item::ReinforcementsPerCall => (
                0,
                i32::from(tenchu_core::roster::ReinforcementCall::MAX_COUNT),
                1,
                1,
            ),
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
            Item::FlameMotion | Item::HeatShimmer => (0, 100, 5, 1),
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
            Self::BurningSpatial => &mut settings.burning_sound.spatial,
            Self::Bleeding => &mut settings.bleeding.enabled,
            Self::Dismemberment => &mut settings.dismemberment.enabled,
            Self::SaExecutionScene => &mut settings.dismemberment.execution_scene,
            Self::PlayerDismemberment => &mut settings.dismemberment.player,
            Self::MedicineBleeding => &mut settings.bleeding.medicine,
            Self::CameraRelative => &mut settings.navigation.camera_relative,
            Self::AlignAim => &mut settings.navigation.align_aim,
            Self::InvertCameraY => &mut settings.navigation.invert_y,
            Self::ModelMoveset => &mut settings.model_moveset,
            Self::ModelPlayerMovement => &mut settings.model_moveset_options.player_movement,
            Self::ModelNoIdleTaunts => &mut settings.model_moveset_options.no_idle_taunts,
            Self::Vibration => &mut settings.frontend.rumble,
            Self::GrappleDetail => &mut settings.graphics.models.grapple,
            Self::HeroModels => &mut settings.graphics.models.heroes,
            Self::ContactSparks => &mut settings.graphics.effects.sparks,
            Self::Lightning => &mut settings.graphics.effects.lightning,
            Self::StormBolts => &mut settings.graphics.weather.storm_bolts,
            Self::Gunfire => &mut settings.graphics.effects.gunfire,
            Self::ContactSplashes => &mut settings.graphics.effects.splashes,
            Self::ContactScatter => &mut settings.graphics.effects.scatter,
            Self::FallingLeaves => &mut settings.graphics.effects.leaves,
            Self::HairMotion => &mut settings.graphics.motion.hair,
            Self::ClothMotion => &mut settings.graphics.motion.cloth,
            Self::EquipmentMotion => &mut settings.graphics.motion.equipment,
            Self::VegetationVariation => &mut settings.graphics.models.variation,
            Self::Water => &mut settings.graphics.water.enabled,
            Self::WaterReflective => &mut settings.graphics.water.reflective,
            Self::Footprints => &mut settings.graphics.weather.footprints,
            Self::BakedSnowFootprints => &mut settings.graphics.weather.baked_snow,
            Self::RoofRunoff => &mut settings.graphics.weather.runoff,
            Self::TimeClock => &mut settings.graphics.sky.enabled,
            Self::BloodConform => &mut settings.graphics.blood.conform_retained,
            Self::BloodRetain => &mut settings.graphics.blood.retain_original,
            Self::BloodModels => &mut settings.graphics.blood.model_stains,
            Self::WeaponBloodMetalOnly => &mut settings.graphics.blood.weapon_metal_only,
            Self::BloodWeapons => &mut settings.graphics.blood.weapons,
            Self::BloodRunoff => &mut settings.graphics.blood.runoff,
            Self::BloodPlayerDeath => &mut settings.graphics.blood.player_death,
            Self::Interpolation => &mut settings.graphics.interpolation,
            Self::Retro => &mut settings.retro,
            Self::Post => &mut settings.graphics.post.enabled,
            Self::Filmic => &mut settings.graphics.post.filmic,
            Self::SmoothModels => &mut settings.graphics.smooth_models,
            Self::AssetPack => &mut settings.graphics.asset_pack,
            Self::SharpText => &mut settings.sharp_text,
            Self::EnglishShuriken => &mut settings.english_shuriken,
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
            Self::MusicWhilePaused => &mut settings.music_while_paused,
            Self::CircleConfirm => &mut settings.circle_confirm,
            Self::WaitAfterDeath => &mut settings.wait_after_death,
            Self::LiveDeathScene => &mut settings.live_death_scene,
            Self::ShopLighting => &mut settings.graphics.shop_lighting,
            Self::Fullscreen => &mut settings.fullscreen,
            Self::Vsync => &mut settings.vsync,
            Self::Hud => &mut settings.show_hud,
            Self::Fog => &mut settings.graphics.fog,
            Self::DistantDoors => &mut settings.graphics.distant_doors,
            Self::Textures => &mut settings.graphics.textures,
            Self::Shading => &mut settings.graphics.shading,
            _ => return None,
        })
    }

    fn number(self, settings: &mut Settings) -> Option<&mut u16> {
        Some(match self {
            Self::BleedChance => &mut settings.bleeding.chance,
            Self::SeverStealth => &mut settings.dismemberment.stealth_head,
            Self::PlayerSeverHead => &mut settings.dismemberment.player_head,
            Self::PlayerSeverArms => &mut settings.dismemberment.player_arms,
            Self::PlayerSeverSide => &mut settings.dismemberment.player_right_arm,
            Self::SeveredBlood => &mut settings.graphics.blood.severed,
            Self::SeveredSeconds => &mut settings.graphics.blood.severed_seconds,
            Self::SeverDive => &mut settings.dismemberment.dive_head,
            Self::SeverArms => &mut settings.dismemberment.arms,
            Self::SeverSide => &mut settings.dismemberment.right_arm,
            Self::BleedSeconds => &mut settings.bleeding.seconds,
            Self::BleedDamage => &mut settings.bleeding.damage,
            Self::BandageLoadout => &mut settings.bleeding.bandages,
            Self::RewindHistory => &mut settings.rewind_seconds,
            Self::RewindSpeed => &mut settings.rewind_speed,
            Self::MouseSensitivity => &mut settings.navigation.mouse_sensitivity,
            Self::StickSensitivity => &mut settings.navigation.stick_sensitivity,
            Self::WalkThreshold => &mut settings.walking.threshold,
            Self::StickDeadzone => &mut settings.navigation.deadzone,
            Self::FollowDistance => &mut settings.navigation.distance,
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
            Self::StormLightning => &mut settings.graphics.weather.lightning,
            Self::StormInterval => &mut settings.graphics.weather.lightning_interval,
            Self::StormThunder => &mut settings.graphics.weather.thunder,
            Self::RainVolume => &mut settings.graphics.weather.rain_volume,
            Self::WindVolume => &mut settings.graphics.weather.wind_volume,
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
            Self::WeaponReflections => &mut settings.graphics.weapon_reflections,
            Self::BloodDeathSize => &mut settings.graphics.blood.death_size,
            Self::BloodSeconds => &mut settings.graphics.blood.seconds,
            Self::BloodSpurts => &mut settings.graphics.blood.spurts,
            Self::DeathGush => &mut settings.graphics.blood.death_gush,
            Self::GushSeconds => &mut settings.graphics.blood.gush_seconds,
            Self::WeaponBloodCoverage => &mut settings.graphics.blood.weapon_coverage,
            Self::WeaponBloodFlow => &mut settings.graphics.blood.weapon_flow,
            Self::AlternateDeathChance => &mut settings.alternate_death_chance,
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
            Self::BurningVolume => &mut settings.burning_sound.volume,
            Self::BurningRange => &mut settings.burning_sound.range,
            Self::FogDistance => &mut settings.graphics.fog_distance,
            Self::Exposure => &mut settings.graphics.exposure,
            Self::SkyTint => &mut settings.graphics.lighting.sky_tint,
            Self::DirectionalIntensity => &mut settings.graphics.lighting.directional.intensity,
            Self::DirectionalAzimuth => &mut settings.graphics.lighting.directional.azimuth,
            Self::DirectionalElevation => &mut settings.graphics.lighting.directional.elevation,
            Self::DirectionalRadius => &mut settings.graphics.lighting.directional.radius,
            Self::LightSourceSize => &mut settings.graphics.lighting.source_size,
            Self::FlameMotion => &mut settings.graphics.lighting.flame_motion,
            Self::HeatShimmer => &mut settings.graphics.lighting.heat_shimmer,
            Self::ShadowSoftness => &mut settings.graphics.lighting.softness,
            Self::LightRadius => &mut settings.graphics.lighting.radius,
            Self::LightIntensity => &mut settings.graphics.lighting.intensity,
            Self::LightAmbient => &mut settings.graphics.lighting.ambient,
            _ => return None,
        })
    }

    pub fn change_setting(self, settings: &mut Settings, direction: i32) -> bool {
        if let Some(value) = self.blood_style(settings) {
            *value = value.cycle(direction);
        } else if let Some(value) = self.boolean(settings) {
            *value = !*value;
            if self == Self::LiveDeathScene && *value {
                settings.wait_after_death = true;
            }
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
            match self {
                Item::CircleAction => {
                    settings.circle_action = settings.circle_action.cycle(direction)
                }
                Item::DirectionalColor => {
                    settings.graphics.lighting.directional.color = cycle(
                        settings.graphics.lighting.directional.color,
                        &[0, 1, 2],
                        direction,
                    )
                }
                Item::DirectionalResolution => {
                    settings.graphics.lighting.directional.resolution = u32::from(cycle(
                        settings.graphics.lighting.directional.resolution as u16,
                        &[512, 1024, 2048, 4096],
                        direction,
                    ))
                }
                Item::ShadowCount => {
                    settings.graphics.lighting.shadow_count = cycle(
                        settings.graphics.lighting.shadow_count,
                        &[0, 1, 2, 3, 4, 6, 8],
                        direction,
                    )
                }
                Item::LightCount => {
                    settings.graphics.lighting.count = cycle(
                        settings.graphics.lighting.count,
                        &[0, 1, 2, 4, 8, 16],
                        direction,
                    )
                }
                Item::LightResolution => {
                    settings.graphics.lighting.resolution = u32::from(cycle(
                        settings.graphics.lighting.resolution as u16,
                        &[256, 512, 1024, 2048],
                        direction,
                    ))
                }
                Item::FogTint => {
                    settings.graphics.post.fog_tint =
                        cycle(settings.graphics.post.fog_tint, &[0, 1, 2, 3], direction)
                }
                Item::Temperature => {
                    settings.graphics.post.temperature =
                        (i32::from(settings.graphics.post.temperature) + direction * 5)
                            .clamp(-50, 50) as i16
                }
                Item::ModelDetail => {
                    settings.graphics.model_detail =
                        cycle(settings.graphics.model_detail, &[0, 1, 2], direction)
                }
                Item::CharacterRefinement => {
                    settings.graphics.models.characters =
                        cycle(settings.graphics.models.characters, &[0, 1, 2], direction)
                }
                Item::FoliageDetail => {
                    settings.graphics.models.foliage = cycle(
                        settings.graphics.models.foliage,
                        &[0, 1, 2, 3, 4],
                        direction,
                    )
                }
                Item::WaterDetail => {
                    settings.graphics.water.detail =
                        cycle(settings.graphics.water.detail, &[0, 1, 2], direction)
                }
                Item::Upscale => {
                    settings.graphics.upscale =
                        cycle(settings.graphics.upscale, &[1, 2, 4], direction)
                }
                Item::SurfaceDetail => {
                    settings.graphics.surface_detail =
                        cycle(settings.graphics.surface_detail, &[0, 1, 2], direction)
                }
                Item::Magnification => {
                    settings.graphics.magnification =
                        cycle(settings.graphics.magnification, &[0, 1, 2, 3], direction)
                }
                Item::Filtering => {
                    let next = cycle(
                        settings.graphics.filtering(),
                        &[0, 1, 2, 3, 4, 5, 6],
                        direction,
                    );
                    settings.graphics.set_filtering(next);
                }
                Item::Weather => {
                    let modes = super::WeatherMode::ALL;
                    let index = modes
                        .iter()
                        .position(|&mode| mode == settings.graphics.weather.mode)
                        .unwrap_or(0);
                    settings.graphics.weather.mode =
                        modes[(index as i32 + direction).rem_euclid(modes.len() as i32) as usize];
                }
                Item::WeatherQuality => {
                    settings.graphics.weather.quality =
                        cycle(settings.graphics.weather.quality, &[0, 1, 2], direction)
                }
                Item::ThunderStyle => {
                    let styles = tenchu_audio::ThunderStyle::ALL;
                    let index = styles
                        .iter()
                        .position(|&style| style == settings.graphics.weather.thunder_style)
                        .unwrap_or(0);
                    settings.graphics.weather.thunder_style =
                        styles[(index as i32 + direction).rem_euclid(styles.len() as i32) as usize];
                }
                Item::CutsceneSpeed => {
                    settings.cutscene_speed =
                        cycle(settings.cutscene_speed, &[1, 2, 4, 8, 16, 0], direction)
                }
                _ => return false,
            }
        }
        true
    }

    pub fn reset_setting(self, settings: &mut Settings) -> bool {
        let selection = match self {
            Self::PlayerModel => Some(&mut settings.player_model),
            Self::MovesetModel => Some(&mut settings.moveset_model),
            Self::WeaponModel => Some(&mut settings.weapon_model),
            Self::VoiceModel => Some(&mut settings.voice_model),
            _ => None,
        };
        if let Some(selection) = selection {
            selection.clear();
            return true;
        }
        if let Self::VibrationAction(action) = self {
            settings.vibration.actions.remove(&action);
            return true;
        }
        let mut defaults = Settings::default();
        if let Some(value) = self.blood_style(settings) {
            *value = *self.blood_style(&mut defaults).unwrap();
        } else if let Some(value) = self.boolean(settings) {
            *value = *self.boolean(&mut defaults).unwrap();
        } else if let Some(value) = self.number(settings) {
            *value = *self.number(&mut defaults).unwrap();
        } else {
            match self {
                Item::CircleAction => settings.circle_action = defaults.circle_action,
                Item::DirectionalColor => {
                    settings.graphics.lighting.directional.color =
                        defaults.graphics.lighting.directional.color
                }
                Item::DirectionalResolution => {
                    settings.graphics.lighting.directional.resolution =
                        defaults.graphics.lighting.directional.resolution
                }
                Item::LightCount => {
                    settings.graphics.lighting.count = defaults.graphics.lighting.count
                }
                Item::ShadowCount => {
                    settings.graphics.lighting.shadow_count =
                        defaults.graphics.lighting.shadow_count
                }
                Item::LightResolution => {
                    settings.graphics.lighting.resolution = defaults.graphics.lighting.resolution
                }
                Item::FogTint => settings.graphics.post.fog_tint = defaults.graphics.post.fog_tint,
                Item::Temperature => {
                    settings.graphics.post.temperature = defaults.graphics.post.temperature
                }
                Item::ModelDetail => {
                    settings.graphics.model_detail = defaults.graphics.model_detail
                }
                Item::CharacterRefinement => {
                    settings.graphics.models.characters = defaults.graphics.models.characters
                }
                Item::FoliageDetail => {
                    settings.graphics.models.foliage = defaults.graphics.models.foliage
                }
                Item::WaterDetail => {
                    settings.graphics.water.detail = defaults.graphics.water.detail
                }
                Item::Upscale => settings.graphics.upscale = defaults.graphics.upscale,
                Item::SurfaceDetail => {
                    settings.graphics.surface_detail = defaults.graphics.surface_detail
                }
                Item::Magnification => {
                    settings.graphics.magnification = defaults.graphics.magnification
                }
                Item::Filtering => settings
                    .graphics
                    .set_filtering(defaults.graphics.filtering()),
                Item::CutsceneSpeed => settings.cutscene_speed = defaults.cutscene_speed,
                Item::Weather => settings.graphics.weather.mode = defaults.graphics.weather.mode,
                Item::WeatherQuality => {
                    settings.graphics.weather.quality = defaults.graphics.weather.quality
                }
                Item::ThunderStyle => {
                    settings.graphics.weather.thunder_style =
                        defaults.graphics.weather.thunder_style
                }
                _ => return false,
            }
        }
        true
    }

    fn blood_style(self, settings: &mut Settings) -> Option<&mut super::BloodStyle> {
        let blood = &mut settings.graphics.blood;
        Some(match self {
            Self::BloodHit => &mut blood.hit,
            Self::BloodDrips => &mut blood.drips,
            Self::BloodDeath => &mut blood.death,
            Self::BloodGround => &mut blood.ground,
            Self::GushStyle => &mut blood.gush_style,
            Self::SeveredStyle => &mut blood.severed_style,
            _ => return None,
        })
    }
}

pub(crate) fn cycle<T: Copy + PartialEq>(value: T, choices: &[T], direction: i32) -> T {
    let index = choices.iter().position(|&v| v == value).unwrap_or(0);
    choices[(index as i32 + direction).rem_euclid(choices.len() as i32) as usize]
}

impl Settings {
    pub fn shadow_preview(&mut self) {
        self.graphics.lighting.enabled = true;
        self.graphics.lighting.count = self.graphics.lighting.count.max(1);
        self.graphics.lighting.shadows = true;
        self.graphics.lighting.fires = false;
        self.graphics.lighting.lamps = false;
        self.graphics.lighting.shadow_count = 1;
        self.graphics.lighting.ambient = 25;
        self.graphics.lighting.intensity = 250;
        self.graphics.lighting.radius = 12;
        self.graphics.lighting.resolution = 1024;
        self.graphics.blob_shadows = false;
    }
}
