mod blood;
pub(crate) mod edit;
pub mod menu;
pub mod overlay;
pub mod runtime;
pub use blood::{Blood, Style as BloodStyle};
mod sky;
pub use sky::Sky;
mod weather;
pub use weather::{Weather, WeatherMode};
mod water;
pub use water::Water;
mod models;
pub use models::Models;
mod effects;
pub use effects::Effects;
mod motion;
pub use motion::Motion;
pub mod gameplay;
pub mod library;
mod rendering;
pub(crate) use rendering::Rendering;
mod post;
pub mod presets;
pub use post::Post;
mod vibration;
pub use vibration::Vibration;
mod navigation;
pub use navigation::Navigation;
mod walking;
pub use walking::Walking;

use serde::{Deserialize, Serialize};
mod store;
pub use store::Store;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub touch: crate::touch::Settings,
    pub show_fps: bool,
    pub show_performance: bool,
    pub pause_on_defocus: bool,
    pub pause_in_menu: bool,
    pub music_while_paused: bool,
    pub circle_confirm: bool,
    pub wait_after_death: bool,
    pub live_death_scene: bool,
    pub keep_enemies_active: bool,
    pub independent_attacks: bool,
    pub quick_shuriken_dodge: bool,
    pub strict_corpse_sight: bool,
    pub guard_controls: bool,
    pub guard_draw: bool,
    pub circle_action: tenchu_core::input::CircleAction,
    pub walking: Walking,
    pub sheathing: tenchu_core::game::sheathing::Settings,
    pub bleeding: tenchu_core::game::bleeding::Settings,
    pub dismemberment: tenchu_core::game::dismemberment::Settings,
    pub damage_escalation: tenchu_core::ai::escalation::Settings,
    pub multi_target_attacks: bool,
    pub attack_assist: bool,
    pub behavior_profiles: tenchu_core::ai::profiles::Settings,
    pub reinforcement_limit: u16,
    pub reinforcement_call: tenchu_core::roster::ReinforcementCall,
    pub sight_percent: u16,
    pub retro: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_hud: bool,
    pub sharp_text: bool,
    pub english_shuriken: bool,
    pub volume: u16,
    pub burning_sound: tenchu_core::audio::BurningSound,
    pub vibration: Vibration,
    pub navigation: Navigation,
    pub graphics: Graphics,
    pub cutscene_speed: u16,
    pub rewind_seconds: u16,
    pub rewind_speed: u16,
    pub player_model: String,
    pub model_moveset: bool,
    pub moveset_model: String,
    pub weapon_model: String,
    pub voice_model: String,
    pub alternate_death_chance: u16,
    pub model_moveset_options: tenchu_core::game::moveset::Options,
    pub model_directory: String,
    pub frontend: crate::frontend::Preferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            touch: Default::default(),
            show_fps: false,
            show_performance: false,
            pause_on_defocus: true,
            pause_in_menu: true,
            music_while_paused: false,
            circle_confirm: false,
            wait_after_death: false,
            live_death_scene: false,
            keep_enemies_active: false,
            independent_attacks: false,
            quick_shuriken_dodge: false,
            strict_corpse_sight: false,
            guard_controls: false,
            guard_draw: false,
            circle_action: Default::default(),
            walking: Walking::default(),
            sheathing: Default::default(),
            bleeding: Default::default(),
            dismemberment: Default::default(),
            damage_escalation: Default::default(),
            multi_target_attacks: false,
            attack_assist: false,
            behavior_profiles: Default::default(),
            reinforcement_limit: 0,
            reinforcement_call: Default::default(),
            sight_percent: 100,
            retro: false,
            fullscreen: false,
            vsync: true,
            show_hud: true,
            sharp_text: false,
            english_shuriken: false,
            volume: 100,
            burning_sound: Default::default(),
            vibration: Vibration::default(),
            navigation: Navigation::default(),
            graphics: Graphics::default(),
            cutscene_speed: 1,
            rewind_seconds: 0,
            rewind_speed: 200,
            player_model: String::new(),
            model_moveset: false,
            moveset_model: String::new(),
            weapon_model: String::new(),
            voice_model: String::new(),
            alternate_death_chance: 0,
            model_moveset_options: Default::default(),
            model_directory: String::new(),
            frontend: crate::frontend::Preferences::default(),
        }
    }
}

impl Settings {
    pub fn moveset_id(&self) -> Option<&str> {
        self.model_moveset
            .then_some(if self.moveset_model.is_empty() {
                &self.player_model
            } else {
                &self.moveset_model
            })
    }

    pub fn restore_gameplay(&mut self, saved: &Self, game: &tenchu_core::game::Game) {
        self.wait_after_death = saved.wait_after_death;
        self.live_death_scene = saved.live_death_scene;
        self.navigation.clone_from(&saved.navigation);
        self.cutscene_speed = saved.cutscene_speed;
        self.player_model.clone_from(&saved.player_model);
        self.model_moveset = saved.model_moveset;
        self.moveset_model.clone_from(&saved.moveset_model);
        self.weapon_model.clone_from(&saved.weapon_model);
        self.voice_model.clone_from(&saved.voice_model);
        self.model_moveset_options = saved.model_moveset_options;
        self.keep_enemies_active = game.roster.keep_all_active;
        self.reinforcement_limit = game.roster.reinforcement_limit.unwrap_or(0) as u16;
        self.reinforcement_call = game.roster.reinforcement_call;
        self.independent_attacks = game.intelligence.independent_attacks;
        self.quick_shuriken_dodge = game.intelligence.quick_shuriken_dodge;
        self.strict_corpse_sight = game.intelligence.strict_corpse_sight;
        self.guard_controls = game.intelligence.guard_controls;
        self.guard_draw = game.intelligence.guard_draw;
        self.circle_action = game.intelligence.circle_action;
        self.sheathing = game.player().sheathing.as_ref().map_or_else(
            || tenchu_core::game::sheathing::Settings {
                enabled: false,
                ..saved.sheathing
            },
            |s| s.settings,
        );
        self.bleeding = game.player().bleeding.as_ref().map_or(
            tenchu_core::game::bleeding::Settings {
                enabled: false,
                ..saved.bleeding
            },
            |s| s.settings,
        );
        self.dismemberment = game.combat.dismemberment;
        self.walking = saved.walking;
        self.walking.enabled = game.walking.is_some();
        self.damage_escalation = game.intelligence.damage_escalation;
        self.multi_target_attacks = game.combat.multi_target;
        self.attack_assist = game.combat.attack_assist;
        self.behavior_profiles = game.intelligence.profiles;
        self.sight_percent = game.intelligence.sight_percent;
    }

    pub(crate) fn validate(&mut self) {
        self.bleeding.validate();
        self.dismemberment.validate();
        self.walking.validate();
        self.sheathing.validate();
        self.rewind_seconds = self.rewind_seconds.min(30);
        self.rewind_speed = self.rewind_speed.clamp(25, 800);
        self.alternate_death_chance = self.alternate_death_chance.min(100);
        self.volume = self.volume.min(100);
        self.burning_sound.volume = self.burning_sound.volume.min(100);
        self.burning_sound.range = self.burning_sound.range.clamp(500, 30000);
        self.vibration.validate();
        self.navigation.validate();
        self.sight_percent = self.sight_percent.min(400);
        self.damage_escalation.validate();
        self.reinforcement_limit = self.reinforcement_limit.min(1000);
        self.reinforcement_call.count = self
            .reinforcement_call
            .count
            .min(tenchu_core::roster::ReinforcementCall::MAX_COUNT);
        if ![0, 1, 2, 4, 8, 16].contains(&self.cutscene_speed) {
            self.cutscene_speed = 1;
        }
        self.graphics.validate();
    }

    pub fn rendering(&self) -> std::borrow::Cow<'_, Self> {
        if !self.retro {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut original = self.clone();
        original.retro = false;
        original.graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        original.sharp_text = false;
        original.player_model.clear();
        original.alternate_death_chance = 0;
        std::borrow::Cow::Owned(original)
    }

    pub fn apply_activation(&self, roster: &mut tenchu_core::roster::Roster) {
        roster.keep_all_active = self.keep_enemies_active;
        roster.reinforcement_limit =
            (self.reinforcement_limit != 0).then_some(usize::from(self.reinforcement_limit));
        roster.reinforcement_call = self.reinforcement_call;
    }

    pub fn set_visual_preset(&mut self, index: u16) {
        self.retro = false;
        self.graphics.set_preset(index);
        self.sharp_text = presets::sharp_text(index);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    #[cfg(test)]
    #[serde(skip)]
    pub(crate) rendering: Rendering,
    pub blood: Blood,
    pub weapon_reflections: u16,
    pub shop_lighting: bool,
    pub sky: Sky,
    pub weather: Weather,
    pub water: Water,
    pub models: Models,
    pub motion: Motion,
    pub effects: Effects,
    pub interpolation: bool,
    pub fog: bool,
    pub fog_distance: u16,
    pub distant_doors: bool,
    pub textures: bool,
    pub shading: bool,
    pub exposure: u16,
    pub bilinear: bool,
    pub mipmaps: bool,
    pub anisotropy: u16,
    pub magnification: u16,
    pub surface_detail: u16,
    pub surface_strength: u16,
    pub asset_pack: bool,
    pub upscale: u16,
    pub smooth_models: bool,
    pub model_detail: u16,
    pub samples: u32,
    pub lighting: Lighting,
    pub blob_shadows: bool,
    pub post: Post,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            #[cfg(test)]
            rendering: Rendering::default(),
            blood: Blood::default(),
            weapon_reflections: 0,
            shop_lighting: false,
            sky: Sky::default(),
            weather: Weather::default(),
            water: Water::default(),
            models: Models::default(),
            motion: Motion::default(),
            effects: Effects::default(),
            interpolation: false,
            fog: true,
            fog_distance: 100,
            distant_doors: true,
            textures: true,
            shading: true,
            exposure: 100,
            bilinear: false,
            mipmaps: false,
            anisotropy: 1,
            magnification: 0,
            surface_detail: 0,
            surface_strength: 50,
            asset_pack: true,
            upscale: 1,
            smooth_models: false,
            model_detail: 0,
            samples: 1,
            lighting: Lighting::default(),
            blob_shadows: true,
            post: Post::default(),
        }
    }
}

impl Graphics {
    pub(crate) fn rendering(&self) -> Rendering {
        // Tests retain the old paths for image comparisons; saved settings cannot disable these.
        #[cfg(test)]
        return self.rendering;
        #[cfg(not(test))]
        Rendering::default()
    }

    fn validate(&mut self) {
        self.fog_distance = self.fog_distance.clamp(25, 400);
        self.exposure = self.exposure.clamp(25, 300);
        if ![1, 2, 4, 8, 16].contains(&self.anisotropy) {
            self.anisotropy = 1;
        }
        if ![1, 2, 4].contains(&self.upscale) {
            self.upscale = 1;
        }
        self.model_detail = self.model_detail.min(2);
        self.magnification = self.magnification.min(3);
        self.surface_detail = self.surface_detail.min(2);
        self.surface_strength = self.surface_strength.min(100);
        self.post.validate();
        self.blood.validate();
        self.weapon_reflections = self.weapon_reflections.min(200);
        self.sky.validate();
        self.weather.validate();
        self.water.validate();
        self.models.validate();
        self.motion.validate();
        self.effects.validate();
        if ![1, 2, 4, 8, 16].contains(&self.samples) {
            self.samples = 1;
        }
        self.lighting.validate();
    }

    pub fn animated_presentation(&self) -> bool {
        self.interpolation
            || self.sky.enabled && self.sky.speed != 0
            || self.weather.mode.precipitates() && self.weather.amount != 0
            || self.water.enabled && self.water.flow != 0
            || self.motion.actors()
            || self.motion.wind != 0
            || self.effects.active()
    }

    pub fn filtering(&self) -> u16 {
        if self.mipmaps {
            2 + self.anisotropy.ilog2() as u16
        } else {
            u16::from(self.bilinear)
        }
    }

    pub fn set_filtering(&mut self, level: u16) {
        self.bilinear = level != 0;
        self.mipmaps = level >= 2;
        self.anisotropy = 1 << level.saturating_sub(2).min(4);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lighting {
    pub enabled: bool,
    pub shadows: bool,
    pub fires: bool,
    pub radius: u16,
    pub intensity: u16,
    pub ambient: u16,
    pub resolution: u32,
    pub count: u16,
    pub shadow_count: u16,
    pub lamps: bool,
    pub markers: bool,
    pub softness: u16,
    pub source_size: u16,
    pub flame_motion: u16,
    pub heat_shimmer: u16,
    pub sky_tint: u16,
    pub directional: Directional,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            enabled: false,
            shadows: true,
            fires: true,
            radius: 12,
            intensity: 150,
            ambient: 60,
            resolution: 512,
            count: 2,
            shadow_count: 2,
            lamps: true,
            markers: false,
            softness: 0,
            source_size: 100,
            flame_motion: 0,
            heat_shimmer: 0,
            sky_tint: 0,
            directional: Directional::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Directional {
    pub intensity: u16,
    pub color: u16,
    pub azimuth: u16,
    pub elevation: u16,
    pub radius: u16,
    pub resolution: u32,
}

impl Default for Directional {
    fn default() -> Self {
        Self {
            intensity: 0,
            color: 1,
            azimuth: 315,
            elevation: 45,
            radius: 30,
            resolution: 2048,
        }
    }
}

impl Lighting {
    fn validate(&mut self) {
        self.radius = self.radius.clamp(2, 50);
        self.intensity = self.intensity.min(400);
        self.ambient = self.ambient.min(100);
        self.softness = self.softness.min(8);
        self.source_size = self.source_size.min(400);
        self.flame_motion = self.flame_motion.min(100);
        self.heat_shimmer = self.heat_shimmer.min(100);
        self.sky_tint = self.sky_tint.min(100);
        self.directional.validate();
        self.count = self.count.min(crate::lighting::MAX_LIGHTS as u16);
        self.shadow_count = self
            .shadow_count
            .min(crate::lighting::MAX_SHADOW_LIGHTS as u16);
        if ![256, 512, 1024, 2048].contains(&self.resolution) {
            self.resolution = 512;
        }
    }
}

impl Directional {
    fn validate(&mut self) {
        self.intensity = self.intensity.min(200);
        self.color = self.color.min(2);
        self.azimuth %= 360;
        self.elevation = self.elevation.clamp(10, 85);
        self.radius = self.radius.clamp(10, 60);
        if ![512, 1024, 2048, 4096].contains(&self.resolution) {
            self.resolution = 2048;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn retro_is_a_rendering_view_and_restores_all_preferences() {
        let mut settings = Settings {
            keep_enemies_active: true,
            sight_percent: 125,
            cutscene_speed: 4,
            volume: 25,
            player_model: "pack:custom".into(),
            model_moveset: true,
            moveset_model: "disc:135".into(),
            alternate_death_chance: 50,
            model_moveset_options: tenchu_core::game::moveset::Options {
                player_movement: true,
                no_idle_taunts: true,
            },
            ..Default::default()
        };
        settings.set_visual_preset(presets::MODERN_HIGH);
        settings.graphics.weather.mode = WeatherMode::Snow;
        let selected = settings.clone();
        settings.retro = true;
        let original = settings.rendering();
        assert_eq!(
            original.graphics,
            Graphics {
                asset_pack: false,
                ..Default::default()
            }
        );
        assert_eq!(original.sight_percent, 125);
        assert!(original.keep_enemies_active);
        assert_eq!(original.cutscene_speed, 4);
        assert_eq!(original.volume, 25);
        assert!(original.player_model.is_empty());
        assert_eq!(original.moveset_model, settings.moveset_model);
        assert_eq!(original.alternate_death_chance, 0);
        assert_eq!(
            original.model_moveset_options,
            settings.model_moveset_options
        );
        let stored = serde_json::to_vec(&settings).unwrap();
        settings = serde_json::from_slice(&stored).unwrap();
        settings.retro = false;
        assert_eq!(settings, selected);
        assert!(matches!(
            settings.rendering(),
            std::borrow::Cow::Borrowed(_)
        ));
        settings.set_visual_preset(presets::MODERN_ULTRA);
        assert_eq!(settings.graphics.weather.mode, WeatherMode::Snow);
        assert_eq!(settings.sight_percent, 125);
        assert_eq!(settings.alternate_death_chance, 50);
        assert!(menu::Item::AlternateDeathChance.reset_setting(&mut settings));
        assert_eq!(settings.alternate_death_chance, 0);
    }

    #[test]
    fn settings_upgrade_and_atomic_replacement_do_not_touch_campaign_saves() {
        let root = std::env::temp_dir().join(format!("tenchu-settings-{}", std::process::id()));
        let store = Store {
            path: root.join("config/settings.json"),
        };
        let mut initial = Settings::default();
        initial.set_visual_preset(presets::MODERN_LOW);
        assert_eq!(store.load().unwrap(), initial);
        let settings = Settings {
            show_fps: true,
            pause_on_defocus: false,
            volume: 25,
            ..Settings::default()
        };
        store.write(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
        fs::write(&store.path, br#"{"show_fps":true,"volume":200,"future":1}"#).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.show_fps && loaded.pause_on_defocus);
        assert_eq!(loaded.volume, 100);
        assert_eq!(loaded.graphics.lighting, Lighting::default());
        assert_eq!(loaded.reinforcement_call, Default::default());
        fs::write(
            &store.path,
            br#"{"reinforcement_call":{"count":65535,"random":true}}"#,
        )
        .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(
            loaded.reinforcement_call.count,
            tenchu_core::roster::ReinforcementCall::MAX_COUNT
        );
        assert!(loaded.reinforcement_call.random);
        fs::write(&store.path, br#"{"cutscene_speed":7,"graphics":{"lighting":{"enabled":true,"count":99,"radius":0,"intensity":65535,"ambient":200,"resolution":99999}}}"#).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.cutscene_speed, 1);
        assert_eq!(
            loaded.graphics.lighting,
            Lighting {
                enabled: true,
                count: crate::lighting::MAX_LIGHTS as u16,
                radius: 2,
                intensity: 400,
                ambient: 100,
                ..Lighting::default()
            }
        );
        fs::write(&store.path, br#"{"graphics":{"rendering":{"half_resolution_ao":false,"adaptive_shadows":false,"cached_blood":false,"quarter_resolution_shadows":false},"weather":{"mode":"Rain"}}}"#).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.graphics.rendering(), Rendering::default());
        assert_eq!(loaded.graphics.weather.mode, WeatherMode::Rain);
        store.write(&loaded).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&store.path).unwrap()).unwrap();
        assert!(saved["graphics"].get("rendering").is_none());
        fs::write(&store.path, b"{").unwrap();
        assert!(store.load().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
