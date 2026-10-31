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
        if item.reset_setting(&mut self.settings) {
            return Ok(());
        }
        let s = Settings::default();
        let d = crate::debug::Debug::new(&self.campaign.game)?;
        match item {
            Item::GodMode => self.set_god_mode(false)?,
            Item::Stealth => self.set_stealth(false)?,
            Item::ClickTeleport => self.debug.click_teleport = false,
            Item::Inventory(slot) => self.set_inventory(slot, 0)?,
            Item::Armor => {
                if self.debug.armor {
                    self.apply_item(Item::Armor, 1)?;
                }
            }
            Item::CharacterRefinement => {
                self.settings.graphics.models.characters = s.graphics.models.characters
            }
            Item::FoliageDetail => {
                self.settings.graphics.models.foliage = s.graphics.models.foliage
            }
            Item::WaterDetail => self.settings.graphics.water.detail = s.graphics.water.detail,
            Item::Weather => self.settings.graphics.weather.mode = s.graphics.weather.mode,
            Item::WeatherQuality => {
                self.settings.graphics.weather.quality = s.graphics.weather.quality
            }
            Item::PlayerModel => self.settings.player_model.clear(),
            Item::GameplayPreset => self.set_gameplay_preset(0)?,
            Item::Preset => self.settings.set_visual_preset(0),
            Item::PhotoMode => self.debug.photo_mode = false,
            Item::Temperature => {
                self.settings.graphics.post.temperature = s.graphics.post.temperature
            }
            Item::FogTint => self.settings.graphics.post.fog_tint = s.graphics.post.fog_tint,
            Item::ModelDetail => self.settings.graphics.model_detail = s.graphics.model_detail,
            Item::Upscale => self.settings.graphics.upscale = s.graphics.upscale,
            Item::SurfaceDetail => {
                self.settings.graphics.surface_detail = s.graphics.surface_detail
            }
            Item::Magnification => self.settings.graphics.magnification = s.graphics.magnification,
            Item::Filtering => self.settings.graphics.set_filtering(s.graphics.filtering()),
            Item::Antialiasing => self.settings.graphics.samples = s.graphics.samples,
            Item::DirectionalColor => {
                self.settings.graphics.lighting.directional.color =
                    s.graphics.lighting.directional.color
            }
            Item::DirectionalResolution => {
                self.settings.graphics.lighting.directional.resolution =
                    s.graphics.lighting.directional.resolution
            }
            Item::LightCount => self.settings.graphics.lighting.count = s.graphics.lighting.count,
            Item::ShadowCount => {
                self.settings.graphics.lighting.shadow_count = s.graphics.lighting.shadow_count
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
            Item::ReinforcementLimit => self.set_reinforcement_limit(s.reinforcement_limit),
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
