use super::{Player, section};
use crate::{
    debug::Debug,
    settings::{Settings, WeatherMode},
};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    ModernPresets,
    Retro,
    GameplayProfiles,
    SnowDrift,
    SnowTracks,
    ClearWeather,
    SightBounds,
    DaylightLamps,
    AreaLamps,
    ShadowCoverage,
    Botanical,
    TreeVariation,
    BranchWind,
    SeasonalFall,
    Grapple,
    WaterContact,
    SnowScatter,
    PuddleScatter,
}

pub fn animate(feature: Feature, player: &Player, settings: &mut Settings, debug: &mut Debug) {
    let p = player.progress();
    if feature == Feature::ModernPresets {
        use crate::settings::presets::{MODERN_HIGH, MODERN_ULTRA, SHOWCASE};
        settings.set_visual_preset([MODERN_HIGH, MODERN_ULTRA, SHOWCASE][section(p, 3)]);
        return;
    }
    let g = &mut settings.graphics;
    let after = p >= 0.5;
    match feature {
        Feature::ModernPresets => unreachable!(),
        Feature::Retro => settings.retro = section(p, 3) == 1,
        Feature::GameplayProfiles => {
            settings.pause_in_menu = false;
            debug.labels = true;
            debug.patrols = true;
        }
        Feature::SnowDrift | Feature::SnowTracks | Feature::ClearWeather | Feature::SnowScatter => {
            g.sky.time = 600;
            g.weather.mode = if feature == Feature::ClearWeather && after {
                WeatherMode::Clear
            } else {
                WeatherMode::Snow
            };
            g.weather.snow = 100;
            g.weather.amount = 100;
            g.weather.wind = if feature == Feature::SnowDrift { 60 } else { 0 };
            g.weather.heading = 45;
            g.weather.quality = 2;
            g.weather.snow_drift = if feature == Feature::SnowDrift && !after {
                0
            } else {
                85
            };
            g.weather.footprint_seconds = 8;
            g.effects.scatter = true;
        }
        Feature::PuddleScatter => {
            g.sky.time = 540;
            g.weather.mode = WeatherMode::Rain;
            g.weather.amount = 75;
            g.weather.puddles = 100;
            g.weather.pooling = 1000;
            g.weather.wetness = 100;
            g.weather.haze = 0;
            g.weather.reflections = 160;
            g.effects.scatter = true;
        }
        Feature::SightBounds => {
            g.sky.time = 600;
            debug.sight = true;
            debug.labels = true;
            debug.xray = false;
        }
        Feature::DaylightLamps => {
            g.sky.enabled = true;
            g.sky.time = if section(p, 3) == 1 { 720 } else { 1320 };
        }
        Feature::AreaLamps => {
            g.sky.enabled = false;
            g.lighting.directional.intensity = 0;
            g.lighting.source_size = if after { 125 } else { 0 };
        }
        Feature::ShadowCoverage => {
            g.lighting.radius = 24;
            g.lighting.shadow_count = 6;
        }
        Feature::Botanical => {
            g.sky.time = 600;
            g.models.foliage = if after { 3 } else { 0 };
            g.models.variation = false;
        }
        Feature::TreeVariation => {
            g.sky.time = 600;
            g.models.variation = after;
        }
        Feature::BranchWind => {
            g.sky.time = 600;
            g.motion.wind = if after { 75 } else { 0 };
        }
        Feature::SeasonalFall => {
            g.sky.time = 600;
            g.effects.leaves = true;
            g.effects.leaf_density = 150;
            g.effects.scatter = true;
            g.motion.wind = 15;
        }
        Feature::Grapple => {
            g.sky.time = 720;
            g.sky.day_ambient = 80;
            g.models.grapple = after;
            g.effects.sparks = after;
            g.effects.spark_density = 130;
            g.lighting.intensity = 100;
        }
        Feature::WaterContact => {
            g.sky.time = 600;
            g.effects.splashes = after;
        }
    }
}

pub fn buttons(feature: Feature, tick: u64, seconds: u16) -> u16 {
    use tenchu_core::input::{CROSS, R1, TRIANGLE, UP};
    match feature {
        Feature::SnowTracks => {
            if tick < 45 {
                UP
            } else {
                0
            }
        }
        Feature::SeasonalFall => match tick {
            30..=39 => UP | R1,
            80..=89 => UP,
            130 => CROSS,
            _ => 0,
        },
        Feature::SnowScatter | Feature::PuddleScatter => match tick {
            30..=59 => UP | R1,
            80..=104 => UP,
            130 => CROSS,
            _ => 0,
        },
        Feature::Grapple => {
            if tick % (u64::from(seconds) * 15) < 15 {
                TRIANGLE
            } else {
                0
            }
        }
        Feature::WaterContact => {
            let t = tick % (u64::from(seconds) * 15);
            if (20..60).contains(&t) {
                UP
            } else if t == 85 {
                CROSS
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn readout(feature: Feature, player: &Player, settings: &Settings, debug: &Debug) -> String {
    let after = player.progress() >= 0.5;
    match feature {
        Feature::ModernPresets => format!(
            "{} · {}x MSAA · {} shadowed lamps",
            crate::settings::presets::NAMES[settings.graphics.preset() as usize],
            settings.graphics.samples,
            settings.graphics.lighting.shadow_count
        ),
        Feature::Retro => if settings.retro {
            "F8: Original graphics · gameplay choices retained"
        } else {
            "F8: Your modern graphics restored"
        }
        .into(),
        Feature::GameplayProfiles => crate::settings::gameplay::current(settings, debug)
            .map_or("Custom", |i| crate::settings::gameplay::PRESETS[i].name)
            .into(),
        Feature::SnowDrift => if after {
            "Soft drifting edges · continuous open-ground snow"
        } else {
            "Fixed-direction shelter for comparison"
        }
        .into(),
        Feature::SnowTracks => "Tracks gradually fill and fade beneath fresh snowfall".into(),
        Feature::ClearWeather => if after {
            "Clear weather now: accumulation, particles and tracks removed"
        } else {
            "Settled snow before clearing"
        }
        .into(),
        Feature::SightBounds => "Sight bounds stop at terrain and walls".into(),
        Feature::DaylightLamps => if settings.graphics.sky.time == 720 {
            "Noon: sunlight dominates exposed surfaces"
        } else {
            "Night: warm lantern fill and local shadows"
        }
        .into(),
        Feature::AreaLamps => if after {
            "Diffuse lantern · soft penumbrae · solid cap still occludes"
        } else {
            "Point-source lantern for comparison"
        }
        .into(),
        Feature::ShadowCoverage => {
            "Wider shadow coverage · stable slots · gradual handovers".into()
        }
        Feature::Botanical => if after {
            "Botanical geometry · folded foliage / rounded stalks"
        } else {
            "Original vegetation geometry"
        }
        .into(),
        Feature::TreeVariation => if after {
            "Stable per-placement crown shapes and proportions"
        } else {
            "Repeated original proportions"
        }
        .into(),
        Feature::BranchWind => if after {
            "Anchored roots · branch sway · independent leaf flutter"
        } else {
            "Still foliage"
        }
        .into(),
        Feature::SeasonalFall => "Falling foliage settles, scatters underfoot and fades".into(),
        Feature::Grapple => if after {
            "Round braided rope · refined hook · sparks and impact light"
        } else {
            "Original rope, hook and impact sprites"
        }
        .into(),
        Feature::WaterContact => if after {
            "Droplets, ripples and nearby shore wetting"
        } else {
            "Original water-contact sprites"
        }
        .into(),
        Feature::SnowScatter | Feature::PuddleScatter => {
            "Walk → run → jump: increasing ground disturbance".into()
        }
    }
}
