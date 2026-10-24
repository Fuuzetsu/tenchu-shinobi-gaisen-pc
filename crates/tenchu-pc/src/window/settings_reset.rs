use super::*;

impl Application {
    pub(super) fn reset_selected_setting(&mut self) {
        let item = self.menu.hovered.unwrap_or_else(|| {
            settings::menu::rows(self.menu.page, &self.settings, &self.debug, self.paused)
                [self.menu.row]
                .item
        });
        let previous = self.settings.clone();
        self.menu.notice = match self.reset_item(item) {
            Ok(()) => "Value reset to default".into(),
            Err(error) => error,
        };
        self.menu.edit = None;
        self.apply_settings(&previous);
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn reset_item(&mut self, item: Item) -> Result<()> {
        let s = Settings::default();
        let d = crate::debug::Debug::new(&self.campaign.game)?;
        match item {
            Item::TimeClock => self.settings.graphics.sky.enabled = s.graphics.sky.enabled,
            Item::TimeOfDay => self.settings.graphics.sky.time = s.graphics.sky.time,
            Item::TimeSpeed => self.settings.graphics.sky.speed = s.graphics.sky.speed,
            Item::SunStrength => self.settings.graphics.sky.sun = s.graphics.sky.sun,
            Item::MoonStrength => self.settings.graphics.sky.moon = s.graphics.sky.moon,
            Item::DayAmbient => self.settings.graphics.sky.day_ambient = s.graphics.sky.day_ambient,
            Item::NightAmbient => {
                self.settings.graphics.sky.night_ambient = s.graphics.sky.night_ambient
            }
            Item::SunPath => self.settings.graphics.sky.heading = s.graphics.sky.heading,
            Item::DayRelighting => {
                self.settings.graphics.sky.relighting = s.graphics.sky.relighting
            }
            Item::Blood => self.settings.graphics.blood.enabled = s.graphics.blood.enabled,
            Item::BloodAmount => self.settings.graphics.blood.amount = s.graphics.blood.amount,
            Item::BloodSeconds => self.settings.graphics.blood.seconds = s.graphics.blood.seconds,
            Item::BloodMarks => self.settings.graphics.blood.marks = s.graphics.blood.marks,
            Item::PlayerModel => self.settings.player_model.clear(),
            Item::Interpolation => self.settings.graphics.interpolation = s.graphics.interpolation,
            Item::Preset => self.settings.graphics.set_preset(0),
            Item::PhotoMode => self.debug.photo_mode = false,
            Item::Post => self.settings.graphics.post.enabled = s.graphics.post.enabled,
            Item::Bloom => self.settings.graphics.post.bloom = s.graphics.post.bloom,
            Item::BloomThreshold => {
                self.settings.graphics.post.bloom_threshold = s.graphics.post.bloom_threshold
            }
            Item::Occlusion => self.settings.graphics.post.occlusion = s.graphics.post.occlusion,
            Item::OcclusionRadius => {
                self.settings.graphics.post.occlusion_radius = s.graphics.post.occlusion_radius
            }
            Item::Filmic => self.settings.graphics.post.filmic = s.graphics.post.filmic,
            Item::Contrast => self.settings.graphics.post.contrast = s.graphics.post.contrast,
            Item::Saturation => self.settings.graphics.post.saturation = s.graphics.post.saturation,
            Item::Temperature => {
                self.settings.graphics.post.temperature = s.graphics.post.temperature
            }
            Item::Gamma => self.settings.graphics.post.gamma = s.graphics.post.gamma,
            Item::Vignette => self.settings.graphics.post.vignette = s.graphics.post.vignette,
            Item::Sharpen => self.settings.graphics.post.sharpen = s.graphics.post.sharpen,
            Item::FogTint => self.settings.graphics.post.fog_tint = s.graphics.post.fog_tint,
            Item::Haze => self.settings.graphics.post.haze = s.graphics.post.haze,
            Item::SmoothModels => self.settings.graphics.smooth_models = s.graphics.smooth_models,
            Item::ModelDetail => self.settings.graphics.model_detail = s.graphics.model_detail,
            Item::AssetPack => self.settings.graphics.asset_pack = s.graphics.asset_pack,
            Item::SharpText => self.settings.sharp_text = s.sharp_text,
            Item::Upscale => self.settings.graphics.upscale = s.graphics.upscale,
            Item::SurfaceDetail => {
                self.settings.graphics.surface_detail = s.graphics.surface_detail
            }
            Item::SurfaceStrength => {
                self.settings.graphics.surface_strength = s.graphics.surface_strength
            }
            Item::Magnification => self.settings.graphics.magnification = s.graphics.magnification,
            Item::Performance => self.settings.show_performance = s.show_performance,
            Item::Fps => self.settings.show_fps = s.show_fps,
            Item::Focus => self.settings.pause_on_defocus = s.pause_on_defocus,
            Item::MenuPause => self.settings.pause_in_menu = s.pause_in_menu,
            Item::Fullscreen => self.settings.fullscreen = s.fullscreen,
            Item::Vsync => self.settings.vsync = s.vsync,
            Item::Hud => self.settings.show_hud = s.show_hud,
            Item::Volume => self.settings.volume = s.volume,
            Item::Fog => self.settings.graphics.fog = s.graphics.fog,
            Item::FogDistance => self.settings.graphics.fog_distance = s.graphics.fog_distance,
            Item::Textures => self.settings.graphics.textures = s.graphics.textures,
            Item::Shading => self.settings.graphics.shading = s.graphics.shading,
            Item::BlobShadows => self.settings.graphics.blob_shadows = s.graphics.blob_shadows,
            Item::Exposure => self.settings.graphics.exposure = s.graphics.exposure,
            Item::Filtering => self.settings.graphics.set_filtering(s.graphics.filtering()),
            Item::Antialiasing => self.settings.graphics.samples = s.graphics.samples,
            Item::Lighting => self.settings.graphics.lighting.enabled = s.graphics.lighting.enabled,
            Item::SkyTint => {
                self.settings.graphics.lighting.sky_tint = s.graphics.lighting.sky_tint
            }
            Item::DirectionalIntensity => {
                self.settings.graphics.lighting.directional.intensity =
                    s.graphics.lighting.directional.intensity
            }
            Item::DirectionalColor => {
                self.settings.graphics.lighting.directional.color =
                    s.graphics.lighting.directional.color
            }
            Item::DirectionalAzimuth => {
                self.settings.graphics.lighting.directional.azimuth =
                    s.graphics.lighting.directional.azimuth
            }
            Item::DirectionalElevation => {
                self.settings.graphics.lighting.directional.elevation =
                    s.graphics.lighting.directional.elevation
            }
            Item::DirectionalRadius => {
                self.settings.graphics.lighting.directional.radius =
                    s.graphics.lighting.directional.radius
            }
            Item::DirectionalResolution => {
                self.settings.graphics.lighting.directional.resolution =
                    s.graphics.lighting.directional.resolution
            }
            Item::Shadows => self.settings.graphics.lighting.shadows = s.graphics.lighting.shadows,
            Item::FireLights => self.settings.graphics.lighting.fires = s.graphics.lighting.fires,
            Item::LightCount => self.settings.graphics.lighting.count = s.graphics.lighting.count,
            Item::ShadowSoftness => {
                self.settings.graphics.lighting.softness = s.graphics.lighting.softness
            }
            Item::ShadowCount => {
                self.settings.graphics.lighting.shadow_count = s.graphics.lighting.shadow_count
            }
            Item::LampLights => self.settings.graphics.lighting.lamps = s.graphics.lighting.lamps,
            Item::LightMarkers => {
                self.settings.graphics.lighting.markers = s.graphics.lighting.markers
            }
            Item::LightRadius => {
                self.settings.graphics.lighting.radius = s.graphics.lighting.radius
            }
            Item::LightIntensity => {
                self.settings.graphics.lighting.intensity = s.graphics.lighting.intensity
            }
            Item::LightAmbient => {
                self.settings.graphics.lighting.ambient = s.graphics.lighting.ambient
            }
            Item::LightResolution => {
                self.settings.graphics.lighting.resolution = s.graphics.lighting.resolution
            }
            Item::CutsceneSpeed => self.settings.cutscene_speed = s.cutscene_speed,
            Item::Hitboxes => self.debug.hitboxes = d.hitboxes,
            Item::Sight => self.debug.sight = d.sight,
            Item::SightStance => self.debug.sight_stance = d.sight_stance,
            Item::Collision => self.debug.collision = d.collision,
            Item::CollisionOnly => self.debug.collision_only = d.collision_only,
            Item::Patrols => self.debug.patrols = d.patrols,
            Item::Events => self.debug.events = d.events,
            Item::Labels => self.debug.labels = d.labels,
            Item::Xray => self.debug.xray = d.xray,
            Item::Trail => self.debug.show_trail = d.show_trail,
            Item::FreeCamera => {
                self.debug.camera = None;
                self.debug.teleport_armed = false;
            }
            Item::FlightSpeed => self.debug.flight_speed = d.flight_speed,
            Item::Fov => {
                if let Some(camera) = &mut self.debug.camera {
                    camera.fov = crate::debug::camera::Camera::from_game(&self.campaign.game).fov;
                }
            }
            Item::Actor => self.debug.selected_actor = d.selected_actor,
            Item::InspectActor => self.debug.inspect_actor = d.inspect_actor,
            Item::Target(axis) => self.debug.target[axis] = d.target[axis],
            Item::SnapFloor => self.debug.snap_to_floor = d.snap_to_floor,
            Item::Speed => self.debug.speed = d.speed,
            Item::SightDistance => self.set_sight(d.sight_percent),
            Item::KeepEnemiesActive => self.keep_enemies_active(s.keep_enemies_active),
            Item::DebugLight => {
                self.debug.light = None;
                self.debug.light_follow = false;
            }
            Item::FollowLight => {
                self.debug.light = self.debug.light_position(&self.campaign.game);
                self.debug.light_follow = false;
            }
            Item::LightAxis(axis) => {
                if let Some(light) = &mut self.debug.light {
                    light[axis] = 0.0;
                }
            }
            Item::Pause if !self.faulted => self.paused = false,
            _ => return Err("This action has no value to reset".into()),
        }
        Ok(())
    }
}
