use super::{Feature, Player, Shot, section, sweep};
use crate::{
    debug::{Debug, camera::Camera},
    settings::Settings,
};
use glam::Vec3;
use tenchu_core::game::Game;

pub struct Scene {
    pub target: Vec3,
    pub yaw: f32,
    pub distance: f32,
    pub pitch: f32,
    pub actor: Option<usize>,
    pub player_start: tenchu_core::math::Position,
    pub previous_player: Vec3,
    pub current_player: Vec3,
    pub bodies: Vec<(String, String)>,
    pub samples: Vec<u32>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.0,
            distance: 6000.0,
            pitch: -0.3,
            actor: None,
            player_start: [0; 3].into(),
            previous_player: Vec3::ZERO,
            current_player: Vec3::ZERO,
            bodies: vec![(String::new(), "Original".into())],
            samples: vec![1],
        }
    }
}

fn part(p: f32, count: usize) -> (usize, f32) {
    let index = section(p, count);
    (index, (p * count as f32 - index as f32).clamp(0.0, 1.0))
}

pub fn camera(player: &Player, game: &Game, interpolate: bool) -> Option<Camera> {
    if let Some(path) = &player.clip().camera {
        let mut camera = path.sample(player.progress());
        if path.follow_player {
            let scene = &player.scene;
            let alpha = if interpolate { player.alpha() } else { 1.0 };
            camera.position += scene.previous_player.lerp(scene.current_player, alpha)
                - Vec3::from(scene.player_start.map(|v| v as f32).to_array());
        }
        return Some(camera);
    }
    if matches!(player.clip().shot, Shot::Cinematic | Shot::Gameplay)
        || matches!(
            player.clip().feature,
            Feature::Remake(super::remake::Feature::Grapple | super::remake::Feature::WaterContact)
        )
    {
        return None;
    }
    let p = player.progress();
    let scene = &player.scene;
    let mut target = scene.target;
    if player.clip().shot == Shot::Player
        && !matches!(
            player.clip().feature,
            Feature::Teleport | Feature::Remake(_) | Feature::AttackAssist(_)
        )
    {
        let alpha = if interpolate { player.alpha() } else { 1.0 };
        target += scene.previous_player.lerp(scene.current_player, alpha)
            - Vec3::from(scene.player_start.map(|v| v as f32).to_array());
    }
    if let Some(actor) = scene
        .actor
        .and_then(|id| game.roster.actors.get(id))
        .and_then(Option::as_ref)
    {
        target = Vec3::from(actor.position.map(|v| v as f32).to_array()) - Vec3::Y * 900.0;
        if (player.clip().randomizer.as_ref().is_some_and(|s| s.engage)
            || matches!(
                player.clip().feature,
                Feature::Blood
                    | Feature::WoundSpurts
                    | Feature::Bleeding
                    | Feature::DeathGush
                    | Feature::BloodSnow
                    | Feature::WeaponSparks
                    | Feature::GuardControls
                    | Feature::UnarmedAttacks
                    | Feature::EnemyBehavior(_)
                    | Feature::WeaponFinishes
            ))
            && player.clip().feature
                != Feature::EnemyBehavior(tenchu_core::ai::profiles::Combat::Coward)
        {
            target = (target
                + scene
                    .previous_player
                    .lerp(scene.current_player, player.alpha())
                - Vec3::Y * 700.0)
                * 0.5;
        }
    }
    let mut camera = Camera {
        position: Vec3::ZERO,
        yaw: scene.yaw,
        pitch: scene.pitch,
        fov: 55.0,
    };
    let mut distance = scene.distance;
    match player.clip().feature {
        Feature::EnemyBehavior(tenchu_core::ai::profiles::Combat::Coward) => {
            let escape = target - scene.current_player;
            camera.yaw = escape.x.atan2(escape.z);
            camera.pitch = -0.1;
            distance = 4000.0;
        }
        Feature::GuardControls | Feature::DeathGush | Feature::UnarmedAttacks => target.y += 700.0,
        Feature::HeroModels => camera.yaw += p * 1.6,
        Feature::Randomizer if player.clip().shot != Shot::Guard => {
            camera.yaw += (p - 0.5) * 0.25;
        }
        Feature::Remake(super::remake::Feature::ShadowCoverage) => {
            distance *= 2.5 - sweep(p) * 1.5;
            camera.yaw += (p - 0.5) * 0.25;
        }
        Feature::FreeCamera => {
            camera.yaw += p * 1.5;
            camera.pitch = -0.25 - sweep(p) * 0.5;
            distance *= 1.0 + sweep(p) * 0.7;
        }
        Feature::Photo => match section(p, 4) {
            0 => camera.fov = 40.0 + sweep(p * 4.0) * 50.0,
            1 => {
                camera.pitch = -1.5;
                distance *= 2.0;
            }
            2 => camera.yaw += 0.8,
            _ => {}
        },
        Feature::Performance => camera.yaw += (p * std::f32::consts::TAU).sin() * 0.6,
        _ => {}
    }
    camera.position = target - camera.forward() * distance;
    Some(camera)
}

pub fn animate(player: &Player, settings: &mut Settings, debug: &mut Debug) {
    let p = player.progress();
    let wave = sweep(p);
    let g = &mut settings.graphics;
    if let Some(fog) = player.clip().fog {
        g.fog = fog;
    }
    match player.clip().feature {
        Feature::Randomizer => {}
        Feature::ShopLighting => {
            g.shop_lighting = p >= 0.12;
            g.sky.enabled = true;
            g.sky.time = if p < 0.55 {
                ((p - 0.12).max(0.0) / 0.43 * 1440.0) as u16 % 1440
            } else {
                660
            };
            g.sky.speed = 0;
            g.lighting.enabled = true;
            g.lighting.shadows = true;
            g.lighting.softness = 2;
            g.weather.mode = if p < 0.55 {
                crate::settings::WeatherMode::Clear
            } else if p < 0.78 {
                crate::settings::WeatherMode::Rain
            } else {
                crate::settings::WeatherMode::Snow
            };
            g.weather.amount = 100;
            g.weather.wind = 70;
            g.weather.heading = if p < 0.78 { 90 } else { 270 };
        }
        Feature::Remake(feature) => super::remake::animate(feature, player, settings, debug),
        Feature::Gunfire => {
            settings.player_model = "disc:128".into();
            settings.model_moveset = true;
            settings.moveset_model.clear();
            settings.model_moveset_options.player_movement = true;
            settings.model_moveset_options.no_idle_taunts = true;
            g.effects.gunfire = p >= 0.5;
            g.lighting.enabled = true;
            g.interpolation = true;
        }
        Feature::WeaponFinishes => {
            g.blood = crate::settings::Blood::modern();
            g.blood.weapons = p >= 0.05;
            g.blood.weapon_coverage = if p < 0.25 { 100 } else { 140 };
            g.blood.weapon_flow = if p < 0.25 { 0 } else { 100 };
            g.weapon_reflections = if p >= 0.5 { 100 } else { 0 };
            g.lighting.enabled = true;
            g.interpolation = true;
            g.effects.sparks = true;
            g.sky.enabled = true;
            g.sky.time = 660;
            g.post.vignette = 0;
        }
        Feature::GuardControls => {
            settings.guard_controls = true;
            g.effects.sparks = true;
            g.lighting.enabled = true;
            g.interpolation = true;
        }
        Feature::AnalogWalking => {
            settings.walking.enabled = true;
            g.interpolation = true;
        }
        Feature::Sheathing => {
            settings.sheathing.enabled = true;
            g.interpolation = true;
        }
        Feature::UnarmedAttacks => {
            settings.sheathing.enabled = true;
            settings.sheathing.unarmed = true;
            g.interpolation = true;
            g.blood = crate::settings::Blood::modern();
            g.sky.enabled = true;
            g.sky.time = 660;
        }
        Feature::EnemyBehavior(combat) => {
            settings.behavior_profiles = tenchu_core::ai::profiles::Settings {
                enabled: true,
                combat,
                ..Default::default()
            };
            g.interpolation = true;
            g.sky.enabled = true;
            g.sky.time = 660;
        }
        Feature::AttackAssist(enabled) => {
            settings.attack_assist = enabled;
            g.interpolation = true;
            g.blood = crate::settings::Blood::modern();
            g.sky.enabled = true;
            g.sky.time = 660;
        }
        Feature::WeaponSparks => {
            g.effects.sparks = p >= 0.5;
            g.effects.spark_density = 150;
            g.lighting.enabled = true;
            g.interpolation = true;
        }
        Feature::DeathGush => {
            g.blood = crate::settings::Blood::modern();
            g.blood.death_gush = 100;
            g.blood.gush_seconds = 6;
            g.blood.spurts = 0;
            g.blood.drips = crate::settings::BloodStyle::Off;
            g.lighting.enabled = true;
            g.interpolation = true;
            g.sky.enabled = true;
            g.sky.time = 660;
            g.post.vignette = 0;
        }
        Feature::Bleeding => {
            settings.bleeding = tenchu_core::game::bleeding::Settings {
                enabled: true,
                chance: 100,
                seconds: 0,
                damage: 3,
                ..Default::default()
            };
            g.blood = crate::settings::Blood::modern();
            g.interpolation = true;
            g.sky.enabled = true;
            g.sky.time = 660;
        }
        Feature::WoundSpurts => {
            g.blood = crate::settings::Blood::modern();
            g.blood.hit = crate::settings::BloodStyle::Off;
            g.blood.runoff = false;
            g.blood.spurts = if p < 0.5 { 0 } else { 100 };
            g.blood.weapons = false;
            g.blood.seconds = 4;
            g.interpolation = true;
            g.lighting.enabled = true;
            g.sky.enabled = true;
            g.sky.time = 660;
            g.post.vignette = 0;
        }
        Feature::Blood => {
            g.blood = crate::settings::Blood::modern();
            let style = if p < 0.25 {
                crate::settings::BloodStyle::Original
            } else if !(0.5..0.75).contains(&p) {
                crate::settings::BloodStyle::OriginalHd
            } else {
                crate::settings::BloodStyle::Modern
            };
            g.blood.ground = style;
            g.blood.death = style;
            g.blood.drips = style;
            g.blood.retain_original = true;
            if p < 0.75 {
                g.blood.hit = style;
            }
            g.interpolation = true;
            g.lighting.enabled = true;
        }
        Feature::GrenadeLight | Feature::KaenLight => {
            g.lighting.enabled = true;
            g.lighting.fires = p >= 0.5;
            g.lighting.intensity = 140;
            g.lighting.directional.intensity = 0;
            g.lighting.count = 4;
            g.interpolation = true;
        }
        Feature::Experiment => {
            settings.pause_in_menu = false;
            settings.show_hud = true;
            settings.sharp_text = true;
            g.interpolation = true;
        }
        Feature::Presets => g.set_preset([0, 3, 4, 1, 2, 5, 6][section(p, 7)]),
        Feature::Vegetation => g.models.foliage = [0, 1, 2][section(p, 3)],
        Feature::CharacterRefinement => {
            g.models.heroes = false;
            g.models.characters = [0, 1, 2][section(p, 3)];
            g.model_detail = 0;
            g.interpolation = true;
            g.lighting.enabled = true;
            g.sky.enabled = true;
            g.sky.time = 600;
        }
        Feature::HeroModels => {
            g.models.heroes = p >= 0.3;
            g.models.characters = 2;
            g.model_detail = 0;
            g.interpolation = true;
            g.lighting.enabled = true;
            g.sky.enabled = true;
            g.sky.time = 600;
            g.post.vignette = 0;
        }
        Feature::SecondaryMotion => {
            let enabled = section(p, 4) % 2 == 1;
            g.motion.hair = enabled;
            g.motion.cloth = enabled;
            g.motion.equipment = enabled;
            g.motion.strength = 150;
            g.models.characters = 1;
            g.interpolation = true;
            g.lighting.enabled = true;
            g.sky.enabled = true;
            g.sky.time = 600;
        }
        Feature::Breeze => {
            g.motion.wind = (wave * 100.0) as u16;
            g.models.foliage = 1;
            g.sky.enabled = true;
            g.sky.time = 720;
        }
        Feature::Water | Feature::Waterfalls => {
            g.water.enabled = section(p, 4) % 2 == 1;
            g.water.detail = 1;
            if player.clip().feature == Feature::Water {
                g.sky.enabled = true;
                g.sky.time = 570;
                g.water.ripples = 85;
                g.water.reflections = 85;
            }
        }
        Feature::Rain
        | Feature::RainReflections
        | Feature::RainHaze
        | Feature::Rainstorm
        | Feature::Snowstorm
        | Feature::Blizzard
        | Feature::Snow
        | Feature::RainPuddles
        | Feature::RoofDrips
        | Feature::RainWind
        | Feature::SnowWind
        | Feature::SnowFootprints
        | Feature::BloodSnow => {
            g.sky.enabled = true;
            g.sky.time = 720;
            g.weather.mode = if matches!(
                player.clip().feature,
                Feature::Snow | Feature::SnowWind | Feature::SnowFootprints | Feature::BloodSnow
            ) {
                crate::settings::WeatherMode::Snow
            } else {
                crate::settings::WeatherMode::Rain
            };
            g.weather.amount = 100;
            g.weather.rain_volume = 80;
            g.weather.quality = 2;
            g.weather.wind = 30;
            g.weather.heading = 90;
            g.weather.wetness = 90;
            g.weather.snow = 75;
            g.weather.pooling = 600;
            g.weather.puddles = 75;
            g.weather.reflections = 160;
            g.weather.streaks = 130;
            g.weather.runoff_amount = 100;
            g.weather.haze = 15;
            if player.clip().feature == Feature::RainReflections {
                g.sky.time = 540;
                g.weather.haze = 0;
                g.weather.reflections = if p < 0.2 {
                    0
                } else {
                    (sweep((p - 0.2) / 0.8) * 240.0) as u16
                };
            }
            if player.clip().feature == Feature::RainHaze {
                g.weather.haze = (wave * 100.0) as u16;
            }
            g.lighting.enabled = true;
            g.lighting.count = 4;
            g.interpolation = true;
            match player.clip().feature {
                Feature::Blizzard => {
                    g.weather.mode = crate::settings::WeatherMode::Blizzard;
                    g.weather.wind = (25.0 + wave * 50.0) as u16;
                    g.weather.wind_volume = 85;
                    g.sky.time = 1020;
                }
                Feature::Rainstorm | Feature::Snowstorm => {
                    g.weather.mode = if player.clip().feature == Feature::Rainstorm {
                        crate::settings::WeatherMode::Rainstorm
                    } else {
                        crate::settings::WeatherMode::Snowstorm
                    };
                    g.weather.lightning_interval = 8;
                    g.weather.thunder_style = tenchu_audio::ThunderStyle::Rolling;
                    g.weather.lightning = 70;
                    g.weather.wind = 65;
                    g.sky.time = 1200;
                }
                Feature::RainPuddles => {
                    g.weather.pooling = 1000;
                    g.weather.puddles = 100;
                    g.weather.reflections = 200;
                    g.weather.haze = 0;
                    g.sky.time = 540;
                }
                Feature::RoofDrips => {
                    g.weather.streaks = 0;
                    g.weather.haze = 0;
                    g.weather.runoff = p >= 0.2;
                    g.weather.reflections = 100;
                }
                Feature::RainWind | Feature::SnowWind => {
                    g.weather.wind = 100;
                    g.weather.heading = [0, 180, 0][section(p, 3)];
                    g.weather.snow = 100;
                    g.weather.haze = 0;
                    g.weather.reflections = 0;
                }
                Feature::SnowFootprints | Feature::BloodSnow => {
                    g.weather.wind = 0;
                    g.weather.snow = 100;
                    g.weather.footprint_seconds = 10;
                    g.blob_shadows = false;
                    g.blood = if player.clip().feature == Feature::BloodSnow {
                        crate::settings::Blood::modern()
                    } else {
                        Default::default()
                    };
                    g.sky.time = 600;
                }
                _ => {}
            }
        }
        Feature::DayCycle => {
            g.sky.enabled = true;
            g.sky.time = 360;
            g.sky.speed = 72;
            g.lighting.enabled = true;
            g.lighting.count = 4;
        }
        Feature::Lanterns => {
            g.lighting.enabled = true;
            g.lighting.intensity = (wave * 220.0).round() as u16;
            g.lighting.directional.intensity = 0;
        }
        Feature::FlameMotion => {
            g.lighting.enabled = true;
            g.lighting.shadows = true;
            g.post.enabled = true;
            g.lighting.flame_motion = match section(p, 4) {
                2 => 100,
                3 => 35,
                _ => 0,
            };
            g.lighting.heat_shimmer = match section(p, 4) {
                1 => 100,
                3 => 25,
                _ => 0,
            };
        }
        Feature::FireLights => {
            g.lighting.enabled = true;
            g.lighting.lamps = false;
            g.lighting.fires = section(p, 4) % 2 == 1;
            g.lighting.directional.intensity = 0;
            g.lighting.intensity = 220;
        }
        Feature::Shadows => {
            g.lighting.directional.intensity = 0;
            g.lighting.shadows = section(p, 3) > 0;
            g.lighting.softness = if section(p, 3) == 2 { 4 } else { 0 };
        }
        Feature::Moon => {
            g.lighting.directional.intensity = (wave * 100.0).round() as u16;
        }
        Feature::TextureSampling => {
            g.set_filtering(6);
            g.magnification = [3, 0, 1, 2][section(p, 4)];
        }
        Feature::Reconstruction => {
            g.upscale = [1, 2, 4][section(p, 3)];
            g.magnification = 3;
        }
        Feature::DistanceFiltering => {
            g.set_filtering(section(p, 7) as u16);
            g.magnification = 0;
        }
        Feature::Antialiasing => {
            g.samples = player.scene.samples[section(p, player.scene.samples.len())];
        }
        Feature::Surfaces => {
            let (stage, t) = part(p, 3);
            g.textures = stage != 1;
            g.shading = stage != 0;
            g.exposure = if stage == 2 {
                100 + (sweep(t) * 100.0) as u16
            } else {
                100
            };
        }
        Feature::ModelDetail => {
            let stage = section(p, 3);
            g.smooth_models = stage > 0;
            g.model_detail = stage as u16;
        }
        Feature::Interpolation => {
            g.interpolation = section(p, 4) % 2 == 1;
        }
        Feature::PlayerModels => {
            let count = player.scene.bodies.len().min(4);
            settings
                .player_model
                .clone_from(&player.scene.bodies[section(p, count)].0);
        }
        Feature::ContactShading => {
            let (stage, t) = part(p, 2);
            g.post.enabled = true;
            g.post.occlusion = if stage == 0 {
                (sweep(t) * 100.0) as u16
            } else {
                65
            };
            g.post.occlusion_radius = if stage == 1 {
                2 + (sweep(t) * 18.0) as u16
            } else {
                8
            };
        }
        Feature::Bloom => {
            let (stage, t) = part(p, 2);
            g.post.enabled = true;
            g.post.bloom = if stage == 0 {
                (sweep(t) * 65.0) as u16
            } else {
                45
            };
            g.post.bloom_threshold = if stage == 1 {
                25 + (sweep(t) * 175.0) as u16
            } else {
                100
            };
        }
        Feature::Atmosphere => {
            let (stage, t) = part(p, 3);
            g.fog = stage != 0;
            g.fog_distance = 25 + (sweep(t) * 375.0) as u16;
            g.post.haze = if stage == 2 {
                (sweep(t) * 80.0) as u16
            } else {
                0
            };
            g.post.fog_tint = if stage == 2 { section(t, 4) as u16 } else { 0 };
        }
        Feature::ColorGrading => {
            let (stage, t) = part(p, 3);
            g.post.enabled = true;
            g.post.temperature = if stage == 0 {
                (sweep(t) * 80.0) as i16 - 40
            } else {
                0
            };
            g.post.saturation = if stage == 1 {
                (sweep(t) * 200.0) as u16
            } else {
                100
            };
            g.post.vignette = if stage == 2 {
                (sweep(t) * 70.0) as u16
            } else {
                0
            };
        }
        Feature::ImageFinish => {
            let (stage, t) = part(p, 5);
            let w = sweep(t);
            g.post.enabled = true;
            g.post.filmic = stage != 0 || t >= 0.5;
            g.post.contrast = if stage == 1 {
                50 + (w * 100.0) as u16
            } else {
                100
            };
            g.post.gamma = if stage == 2 {
                50 + (w * 100.0) as u16
            } else {
                100
            };
            g.post.sharpen = if stage == 3 { (w * 100.0) as u16 } else { 0 };
            g.post.enabled = stage != 4 || t >= 0.5;
        }
        Feature::Inspection => {
            let stage = section(p, 4);
            debug.hitboxes = stage == 0;
            debug.labels = true;
            debug.sight = stage == 1;
            debug.patrols = stage >= 2;
            debug.inspect_actor = stage == 3;
            debug.xray = stage >= 2;
        }
        Feature::Collision => {
            let stage = section(p, 3);
            debug.collision = true;
            debug.collision_only = stage == 1;
            debug.events = stage == 2;
            debug.xray = stage == 2;
        }
        Feature::EnemySimulation => {
            settings.keep_enemies_active = section(p, 3) == 1;
            debug.labels = true;
            debug.patrols = true;
            debug.xray = true;
            debug.inspect_actor = true;
        }
        Feature::SightDistance => {
            debug.sight = true;
            debug.labels = true;
            let (stage, t) = part(p, 2);
            debug.sight_percent = if stage == 0 {
                25 + (sweep(t) * 175.0) as u16
            } else {
                100
            };
            debug.sight_stance = if stage == 1 {
                1 + section(t, 2) as u8
            } else {
                1
            };
        }
        Feature::Simulation => {
            debug.speed = player.speed();
        }
        Feature::Cinematics => {
            settings.cutscene_speed = player.speed() / 100;
        }
        Feature::Performance => {
            settings.show_performance = true;
            settings.show_fps = true;
            g.fog = false;
        }
        Feature::Artwork => {
            g.asset_pack = player.clip().artwork.unwrap_or(section(p, 4) % 2 == 1);
        }
        Feature::LightRange => {
            let (stage, t) = part(p, 2);
            g.lighting.radius = if stage == 0 {
                2 + (sweep(t) * 18.0) as u16
            } else {
                10
            };
            g.lighting.ambient = if stage == 1 {
                (sweep(t) * 100.0) as u16
            } else {
                25
            };
        }
        Feature::LightBudget => {
            let stage = section(p, 4);
            g.lighting.markers = true;
            g.lighting.count = [1, 4, 8, 4][stage];
            g.lighting.shadow_count = [1, 2, 4, 4][stage];
            g.lighting.lamps = stage != 3;
            g.lighting.fires = true;
        }
        Feature::MovableLight => {
            g.lighting.directional.intensity = 0;
            g.lighting.lamps = false;
            g.lighting.fires = false;
            g.lighting.markers = true;
            let a = p * std::f32::consts::TAU;
            debug.light =
                Some(player.scene.target + Vec3::new(a.sin() * 2500.0, -2000.0, a.cos() * 2500.0));
        }
        Feature::ShadowQuality => {
            let (stage, t) = part(p, 3);
            g.lighting.resolution = if stage == 0 {
                [256, 512, 1024, 2048][section(t, 4)]
            } else {
                1024
            };
            g.lighting.softness = if stage == 1 {
                (sweep(t) * 8.0) as u16
            } else {
                0
            };
            g.blob_shadows = stage == 2 && t < 0.5;
            g.lighting.shadows = stage != 2 || t >= 0.5;
        }
        Feature::SkyDirection => {
            let (stage, t) = part(p, 4);
            g.lighting.directional.intensity = 100;
            g.lighting.directional.azimuth = if stage == 0 { (t * 359.0) as u16 } else { 315 };
            g.lighting.directional.elevation = if stage == 1 {
                10 + (sweep(t) * 75.0) as u16
            } else {
                45
            };
            g.lighting.sky_tint = if stage == 2 {
                (sweep(t) * 100.0) as u16
            } else {
                25
            };
            g.lighting.directional.color = if stage == 3 { section(t, 3) as u16 } else { 1 };
        }
        Feature::SkyQuality => {
            let (stage, t) = part(p, 2);
            g.lighting.directional.radius = if stage == 0 {
                10 + (sweep(t) * 50.0) as u16
            } else {
                30
            };
            g.lighting.directional.resolution = if stage == 1 {
                [512, 1024, 2048, 4096][section(t, 4)]
            } else {
                2048
            };
        }
        Feature::Teleport => {
            debug.show_trail = true;
            debug.labels = true;
            debug.inspect_actor = true;
        }
        Feature::Photo => {
            debug.photo_mode = section(p, 4) == 3;
            settings.show_hud = !debug.photo_mode;
        }
        Feature::Interface => {
            let stage = section(p, 3);
            settings.pause_in_menu = false;
            settings.show_hud = stage != 2;
            settings.show_fps = stage > 0;
        }
        Feature::FreeCamera | Feature::Step => {}
    }
}

pub struct Readout {
    pub label: String,
    pub fraction: Option<f32>,
}

pub fn readout(player: &Player, s: &Settings, d: &Debug) -> Readout {
    let p = player.progress();
    let g = &s.graphics;
    let l = &g.lighting;
    let post = &g.post;
    let meter = |label: &str, value: u16, max: u16| Readout {
        label: format!("{label}: {value}"),
        fraction: Some(f32::from(value) / f32::from(max)),
    };
    let text = |label: String| Readout {
        label,
        fraction: None,
    };
    let on = |yes| if yes { "On" } else { "Off" };
    match player.clip().feature {
        Feature::Randomizer => {
            let (run, plan) = player.randomized.as_ref().unwrap();
            let body = plan
                .player_model
                .as_ref()
                .and_then(|id| player.scene.bodies.iter().find(|(key, _)| key == id))
                .map_or(
                    if plan.character == 0 {
                        "Rikimaru"
                    } else {
                        "Ayame"
                    },
                    |(_, name)| name.as_str(),
                );
            text(format!(
                "{} · {} HP · seed: {} · roll {}",
                body,
                plan.health.map_or("original".into(), |hp| hp.to_string()),
                run.document.seed,
                u64::from(run.round) + 1
            ))
        }
        Feature::Remake(feature) => text(super::remake::readout(feature, player, s, d)),
        Feature::WeaponFinishes => text(if !s.graphics.blood.weapons {
            "Clean weapon surfaces".into()
        } else if s.graphics.blood.weapon_flow == 0 {
            "Previous static coating · tip drips".into()
        } else if s.graphics.weapon_reflections == 0 {
            "Fuller wet coating · gravity-driven streaks · drips from the lowest edge".into()
        } else {
            "Metal reflects sky and lights · player and enemy weapons".into()
        }),
        Feature::WeaponSparks => text(if s.graphics.effects.sparks {
            "Modern sparks · hot trails, sharp flashes and moving spark light".into()
        } else {
            "Original weapon-block and clash particles".into()
        }),
        Feature::Gunfire => text(
            if s.graphics.effects.gunfire {
                "Modern gunfire · muzzle flash and light · smoke and impact sparks"
            } else {
                "Original gun effects · the same attack timing and damage"
            }
            .into(),
        ),
        Feature::ShopLighting => text(if s.graphics.shop_lighting {
            format!(
                "Sky and window shadows · {:02}:{:02} · {} outside",
                s.graphics.sky.time / 60,
                s.graphics.sky.time % 60,
                s.graphics.weather.mode.name()
            )
        } else {
            "Original painted room".into()
        }),
        Feature::Blood => text(format!(
            "{} hit spray · {} wound drips · {} ground spots · retained blood",
            s.graphics.blood.hit.label(),
            s.graphics.blood.drips.label(),
            s.graphics.blood.ground.label(),
        )),
        Feature::BloodSnow => text("Blood stains the snow; the covered ground stays intact".into()),
        Feature::GuardControls => text(match player.simulation_ticks {
            180..270 => "The opponent pauses · recoil finishes before the next action",
            270..330 => "Guard + right: one dash per press · character-relative direction",
            330..360 => "Hold crouch + guard to prepare a roll",
            360..420 => "Crouch + guard + left: one roll · original collision and motion",
            _ => "Hold guard: stand and block frontal melee hits · original recoil · vulnerable from behind",
        }.into()),
        Feature::AnalogWalking => text(match player.simulation_ticks {
            90..120 | 210..240 => "Full stick tilt: original run speed · weapons and combat retained",
            240.. => "F1 → Movement · rebind Walk modifier under General → Controls",
            _ => "Light stick tilt / Walk modifier: borrowed ninja walk cycle and speed · optional gameplay change",
        }.into()),
        Feature::Sheathing => text(match player.simulation_ticks {
            0..100 => "Press Draw / sheathe: original ninja stowing animation · H by default",
            100..190 => "Weapons stay stowed while moving · speed, sight and vulnerability are configurable",
            190..240 => "Attack draws first · stealth kills draw immediately for their paired animation",
            _ => "Armed attacks retain the hero's original moveset · assign a controller button in Controls",
        }.into()),
        Feature::UnarmedAttacks => text(match player.simulation_ticks {
            0..90 => "Put away the blades · Unarmed attacks is a separate optional gameplay setting",
            90..200 => "Onikage's three-kick combo · original limb hitboxes, damage and timing",
            200..300 => "Directional attacks use the available Onikage moves · weapons remain stowed",
            _ => "Draw explicitly to return to the hero's own blade attacks · stealth kills still draw automatically",
        }.into()),
        Feature::EnemyBehavior(combat) => text(match combat {
            tenchu_core::ai::profiles::Combat::Original => "Original authored decisions · same guard and starting positions",
            tenchu_core::ai::profiles::Combat::Coward => "Coward retreats for help · valid ground and terrain cover required · recruits cannot call again",
            tenchu_core::ai::profiles::Combat::Reckless => "Reckless closes distance and attacks · original hitboxes, damage and recovery",
            tenchu_core::ai::profiles::Combat::Cautious => "Cautious backs away and attacks in openings · does not interrupt committed moves",
            tenchu_core::ai::profiles::Combat::Flanker => "Flanker alternates side dashes and attacks · ordinary collision rules",
            _ => "Watchman adapts to distance, health and player attacks · no items · F1 → Enemy behavior",
        }.into()),
        Feature::AttackAssist(enabled) => text(if enabled {
            "Forward assist · ignores the closer guard behind · retains the front target through the combo"
        } else {
            "Original targeting · the closer guard behind is selected · same positions and inputs"
        }.into()),
        Feature::DeathGush => text("Optional death gush · 100% · 6 seconds · follows the original blood cue and animated body".into()),
        Feature::Bleeding => text(match player.simulation_ticks {
            0..150 => "Optional gameplay: enemy hits start bleeding · 3 HP/s in this demonstration",
            150..240 => "Opponent stands down · bleeding continues until treatment · cosmetic blood is independent",
            _ => "Use a bandage: stops blood loss without healing · chance, duration, damage and stock are configurable",
        }.into()),
        Feature::WoundSpurts => text(if s.graphics.blood.spurts == 0 {
            "Drips only · hit spray hidden · 4 seconds per wound".into()
        } else {
            "Pulsing jets follow wounds · weaken over 4 seconds · cosmetic only".into()
        }),
        Feature::Blizzard => meter("Blizzard wind", g.weather.wind, 100),
        Feature::Rainstorm | Feature::Snowstorm => text(
            "Denser precipitation · rain ambience · sheltered flashes · delayed thunder".into(),
        ),
        Feature::GrenadeLight | Feature::KaenLight => text(format!(
            "Fire / explosion lights {} · original item behavior",
            on(l.fires)
        )),
        Feature::Experiment => text(
            match section(p, 4) {
                0 => "F1 → Experiment · refill and edit mission inventory",
                1 => "God mode and never-noticed stealth · optional gameplay tools",
                _ => "Click a collision surface to teleport · no campaign-save changes",
            }
            .into(),
        ),
        Feature::RainPuddles => {
            text("Pooling 10× · shallow puddles and moving rain ripples".into())
        }
        Feature::RoofDrips => text(format!(
            "Eave drips {} · falling rain streaks hidden for clarity",
            on(g.weather.runoff)
        )),
        Feature::RainWind | Feature::SnowWind => text(format!(
            "Wind toward {}° · exposed faces receive weather; cover shields the rest",
            g.weather.heading
        )),
        Feature::SnowFootprints => text(
            if p < 0.2 {
                "Walking leaves grounded tracks in settled snow"
            } else {
                "Fresh snowfall buries the tracks · 10-second burial setting"
            }
            .into(),
        ),
        Feature::Presets => text(crate::settings::presets::NAMES[g.preset() as usize].into()),
        Feature::Vegetation => text(
            [
                "Original foliage",
                "Fuller leaf clusters and refined trunks",
                "Denser leaf clusters and refined trunks",
            ][usize::from(g.models.foliage)]
            .into(),
        ),
        Feature::CharacterRefinement => text(
            [
                "Original character geometry",
                "Refined surfaces · original joints and UV seams",
                "Fine surfaces · original animation and attachments",
            ][usize::from(g.models.characters)]
            .into(),
        ),
        Feature::HeroModels => text(
            if g.models.heroes {
                "High-detail hero · rounded surfaces and finger creases · original rig"
            } else {
                "Existing Fine character surfaces"
            }
            .into(),
        ),
        Feature::Lanterns => meter("Point-light strength (%)", l.intensity, 220),
        Feature::SecondaryMotion => text(format!(
            "Hair, loose cloth and equipment motion {} · original weapons and gameplay",
            on(g.motion.hair)
        )),
        Feature::Breeze => meter("Breeze (%)", g.motion.wind, 100),
        Feature::Rain => text("Roof rivulets · recurring eave drips · shelter-aware rain".into()),
        Feature::RainReflections => meter("Wet reflections (%)", g.weather.reflections, 240),
        Feature::RainHaze => meter("Rain haze (%)", g.weather.haze, 100),
        Feature::Snow => text("Snow · wind · gradual exposed-surface coverage".into()),
        Feature::Water | Feature::Waterfalls => text(if g.water.enabled {
            "Water upgrade · ripples, sky reflections, curved falls and spray".into()
        } else {
            "Original water geometry and shading".into()
        }),
        Feature::Moon => meter("Moon strength (%)", l.directional.intensity, 100),
        Feature::DayCycle => {
            let minutes = (360.0 + player.time() * 72.0) as u32 % 1440;
            text(format!(
                "{:02}:{:02} · moving sun/moon shadows · original game logic",
                minutes / 60,
                minutes % 60
            ))
        }
        Feature::FlameMotion => text([
            "Steady flame lighting · no distortion",
            "Heat shimmer 100% · background only",
            "Flame motion 100% · cached shadow maps",
            "Modern High · gentle shimmer and dancing shadows",
        ][section(p, 4)].into()),
        Feature::FireLights => text(format!("Authored fire lights: {}", on(l.fires))),
        Feature::Shadows => {
            text(["Shadows off", "Hard model shadows", "Soft model shadows"][section(p, 3)].into())
        }
        Feature::TextureSampling => {
            text(["Smooth", "Crisp", "Cubic", "Pixel"][g.magnification as usize].into())
        }
        Feature::Reconstruction => text(format!("{}x texture reconstruction", g.upscale)),
        Feature::DistanceFiltering => text(
            [
                "Nearest",
                "Bilinear",
                "Trilinear mipmaps",
                "2x anisotropic",
                "4x anisotropic",
                "8x anisotropic",
                "16x anisotropic",
            ][g.filtering() as usize]
                .into(),
        ),
        Feature::Antialiasing => text(format!("{}x MSAA · supported by this GPU", g.samples)),
        Feature::Surfaces => text(format!(
            "Textures {} · baked shading {} · exposure {}%",
            on(g.textures),
            on(g.shading),
            g.exposure
        )),
        Feature::ModelDetail => text(format!(
            "{} contours · smooth normals {}",
            ["Original", "Subtle", "Rounded"][g.model_detail as usize],
            on(g.smooth_models)
        )),
        Feature::Interpolation => text(format!(
            "Visual interpolation {} · game logic 30 Hz",
            on(g.interpolation)
        )),
        Feature::PlayerModels => text(
            player
                .scene
                .bodies
                .iter()
                .find(|(id, _)| *id == s.player_model)
                .map_or("Original", |(_, name)| name)
                .into(),
        ),
        Feature::ContactShading => {
            if p < 0.5 {
                meter("Contact shading (%)", post.occlusion, 100)
            } else {
                meter("Contact radius", post.occlusion_radius, 20)
            }
        }
        Feature::Bloom => {
            if p < 0.5 {
                meter("Bloom strength (%)", post.bloom, 65)
            } else {
                meter("Bloom threshold (%)", post.bloom_threshold, 200)
            }
        }
        Feature::Atmosphere => text(format!(
            "Fog {} · distance {}% · haze {}% · tint {}",
            on(g.fog),
            g.fog_distance,
            post.haze,
            ["Black", "Night", "Warm", "Mist"][post.fog_tint as usize]
        )),
        Feature::ColorGrading => text(format!(
            "Warmth {:+} · saturation {}% · vignette {}%",
            post.temperature, post.saturation, post.vignette
        )),
        Feature::ImageFinish => text(format!(
            "Post {} · filmic {} · contrast {} · gamma {} · sharpen {}",
            on(post.enabled),
            on(post.filmic),
            post.contrast,
            post.gamma,
            post.sharpen
        )),
        Feature::Inspection => text(
            [
                "Body and attack hitboxes",
                "Enemy sight volumes",
                "Patrol paths through walls",
                "Live actor state",
            ][section(p, 4)]
            .into(),
        ),
        Feature::Collision => text(
            [
                "Collision surface overlay",
                "Collision geometry only",
                "Event regions and x-ray",
            ][section(p, 3)]
            .into(),
        ),
        Feature::EnemySimulation => text(format!(
            "Keep all enemies active: {}",
            on(s.keep_enemies_active)
        )),
        Feature::SightDistance => text(format!(
            "Sight range {}% · {} bounds",
            d.sight_percent,
            if d.sight_stance == 2 {
                "Crouching"
            } else {
                "Standing"
            }
        )),
        Feature::Simulation => meter("Simulation speed (%)", player.speed(), 400),
        Feature::Cinematics => text(format!(
            "Cutscene setting: {}x · {}",
            player.speed() / 100,
            if player.cinematic_active {
                "Playing original script"
            } else {
                "Normal gameplay at 1x"
            }
        )),
        Feature::Performance => text("Live CPU / GPU timings · F1 to freeze or clear".into()),
        Feature::Artwork
            if player.clip().frontend.as_ref().is_some_and(|scene| {
                matches!(scene.page, crate::frontend::showcase::Menu::Briefing)
            }) =>
        {
            text(format!("High-definition text: {}", on(s.sharp_text)))
        }
        Feature::Artwork if player.clip().artwork.is_some() => text(
            if g.asset_pack {
                "AFTER · Replacement artwork"
            } else {
                "BEFORE · Original artwork"
            }
            .into(),
        ),
        Feature::Artwork => text(format!(
            "Replacement artwork: {} · original fallback when no pack is loaded",
            on(g.asset_pack)
        )),
        Feature::LightRange => {
            if p < 0.5 {
                meter("Light radius (1000 units)", l.radius, 20)
            } else {
                meter("Baked ambient (%)", l.ambient, 100)
            }
        }
        Feature::LightBudget => text(format!(
            "{} lights per region · {} shadowed · lamps {} · fire lights On",
            l.count,
            l.shadow_count,
            on(l.lamps)
        )),
        Feature::MovableLight => {
            text("World-space light orbit · actual model and weapon casters".into())
        }
        Feature::ShadowQuality => text(format!(
            "{}px per cube face · softness {} · blobs {} · model shadows {}",
            l.resolution,
            l.softness,
            on(g.blob_shadows),
            on(l.shadows)
        )),
        Feature::SkyDirection => text(format!(
            "Moon {}° / {}° · sky tint {}% · {}",
            l.directional.azimuth,
            l.directional.elevation,
            l.sky_tint,
            ["White", "Cool", "Warm"][l.directional.color as usize]
        )),
        Feature::SkyQuality => text(format!(
            "Moon coverage {}k units · {}px shadow map",
            l.directional.radius, l.directional.resolution
        )),
        Feature::Teleport => text(
            [
                "Player position · collision floor target",
                "Teleport to nearby floor",
                "Return to original position",
            ][section(p, 3)]
            .into(),
        ),
        Feature::Photo => text(
            [
                "Field of view sweep",
                "Overhead framing",
                "Camera bookmark",
                "Clean photo view · F9 screenshot",
            ][section(p, 4)]
            .into(),
        ),
        Feature::Interface => text(format!(
            "Game HUD {} · clear FPS label {} · F1 preferences are temporary here",
            on(s.show_hud),
            on(s.show_fps)
        )),
        Feature::Step => text(format!(
            "Single-step preview · {} demo game ticks",
            player.simulation_ticks
        )),
        Feature::FreeCamera => text("Free camera orbit · game state is independent".into()),
    }
}
