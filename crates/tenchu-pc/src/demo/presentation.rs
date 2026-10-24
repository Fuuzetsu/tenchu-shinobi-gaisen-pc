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
        return Some(path.sample(player.progress()));
    }
    if matches!(player.clip().shot, Shot::Cinematic | Shot::Gameplay) {
        return None;
    }
    let p = player.progress();
    let scene = &player.scene;
    let mut target = scene.target;
    if player.clip().shot == Shot::Player && player.clip().feature != Feature::Teleport {
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
    }
    let mut camera = Camera {
        position: Vec3::ZERO,
        yaw: scene.yaw,
        pitch: scene.pitch,
        fov: 55.0,
    };
    let mut distance = scene.distance;
    match player.clip().feature {
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
        Feature::Presets => g.set_preset([0, 3, 4, 1, 2, 5, 6][section(p, 7)]),
        Feature::Vegetation => g.models.foliage = [0, 1, 2][section(p, 3)],
        Feature::CharacterRefinement => {
            g.models.characters = [0, 1, 2][section(p, 3)];
            g.model_detail = 0;
            g.interpolation = true;
            g.lighting.enabled = true;
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
        Feature::Rain | Feature::Snow => {
            g.sky.enabled = true;
            g.sky.time = 720;
            g.weather.mode = if player.clip().feature == Feature::Rain {
                crate::settings::WeatherMode::Rain
            } else {
                crate::settings::WeatherMode::Snow
            };
            g.weather.amount = 100;
            g.weather.wind = 30;
            g.weather.heading = 90;
            g.weather.wetness = 90;
            g.weather.snow = 75;
            g.lighting.enabled = true;
            g.lighting.count = 4;
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
        Feature::Lanterns => meter("Point-light strength (%)", l.intensity, 220),
        Feature::SecondaryMotion => text(format!(
            "Hair, loose cloth and equipment motion {} · original weapons and gameplay",
            on(g.motion.hair)
        )),
        Feature::Breeze => meter("Breeze (%)", g.motion.wind, 100),
        Feature::Rain => {
            text("Rain · roof runoff · gradual wetness · shelter-aware surfaces".into())
        }
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
            "{} point lights · {} shadowed · lamps {} · fire lights On",
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
