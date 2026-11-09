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
        if let Some(reason) = settings::runtime::Runtime::Gaisen.unavailable(item) {
            return Err(reason.into());
        }
        if item.reset_setting(&mut self.settings) {
            return Ok(());
        }
        if self.debug.reset_tool(&self.campaign.game, item)? {
            return Ok(());
        }
        let s = Settings::default();
        let d = crate::debug::Debug::new(&self.campaign.game)?;
        match item {
            Item::Tenchu2Missions | Item::Tenchu2Diagnostics | Item::Tenchu2Launcher => {
                unreachable!("checked runtime availability")
            }
            Item::GodMode => self.set_god_mode(false)?,
            Item::Stealth => self.set_stealth(false)?,
            Item::ClickTeleport => self.debug.click_teleport = false,
            Item::Bandages => {
                if let Some(state) = &mut self
                    .campaign
                    .game
                    .roster
                    .get_mut(self.campaign.game.player_id)
                    .bleeding
                {
                    state.bandages = 0;
                }
            }
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
            Item::HeadRest => {
                let mut settings = self.settings.dismemberment;
                settings.head_rest = s.dismemberment.head_rest;
                self.set_dismemberment(settings);
            }
            Item::PlayerModel => self.settings.player_model.clear(),
            Item::MovesetModel => self.settings.moveset_model.clear(),
            Item::GameplayPreset => self.set_gameplay_preset(0)?,
            Item::Preset => self.settings.set_visual_preset(0),
            Item::Antialiasing => self.settings.graphics.samples = s.graphics.samples,
            Item::SightDistance => self.set_sight(d.sight_percent),
            Item::KeepEnemiesActive => self.keep_enemies_active(s.keep_enemies_active),
            Item::IndependentAttacks => self.set_independent_attacks(s.independent_attacks),
            Item::QuickShurikenDodge => self.set_quick_shuriken_dodge(s.quick_shuriken_dodge),
            Item::StrictCorpseSight => self.set_strict_corpse_sight(s.strict_corpse_sight),
            Item::Sheathing
            | Item::UnarmedAttacks
            | Item::UnarmedStyle
            | Item::SheathedSpeed
            | Item::SheathedVisibility
            | Item::SheathedDamage
            | Item::StaySheathed
            | Item::SheathedOneHit => {
                let mut settings = self.settings.sheathing;
                match item {
                    Item::Sheathing => settings.enabled = s.sheathing.enabled,
                    Item::UnarmedAttacks => settings.unarmed = s.sheathing.unarmed,
                    Item::UnarmedStyle => settings.unarmed_style = s.sheathing.unarmed_style,
                    Item::SheathedSpeed => settings.speed = s.sheathing.speed,
                    Item::SheathedVisibility => settings.visibility = s.sheathing.visibility,
                    Item::SheathedDamage => settings.damage = s.sheathing.damage,
                    Item::StaySheathed => settings.stay_sheathed = s.sheathing.stay_sheathed,
                    _ => settings.one_hit = s.sheathing.one_hit,
                }
                self.set_sheathing(settings)?;
            }
            Item::AnalogWalking => self.set_walking(s.walking.enabled)?,
            Item::GuardDraw => {
                self.stop_edited_recording();
                self.settings.guard_draw = s.guard_draw;
                self.campaign.game.intelligence.guard_draw = s.guard_draw;
            }
            Item::GuardControls => self.set_guard_controls(s.guard_controls),
            Item::DetectionDamage => {
                self.set_damage_escalation(tenchu_core::ai::escalation::Settings {
                    increment: s.damage_escalation.increment,
                    ..self.settings.damage_escalation
                })
            }
            Item::DetectionDamageLimit => {
                self.set_damage_escalation(tenchu_core::ai::escalation::Settings {
                    maximum: s.damage_escalation.maximum,
                    ..self.settings.damage_escalation
                })
            }
            Item::BehaviorProfiles
            | Item::BehaviorCombat
            | Item::BehaviorIdle
            | Item::BehaviorQuickTurns
            | Item::BehaviorProgression
            | Item::BehaviorSeed
            | Item::ActorCombat
            | Item::ActorIdle => self.edit_behavior(item, 0, true)?,
            Item::AttackAssist => self.set_attack_assist(s.attack_assist),
            Item::MultiTargetAttacks => self.set_multi_target_attacks(s.multi_target_attacks),
            Item::ReinforcementsPerCall => {
                self.set_reinforcement_call(tenchu_core::roster::ReinforcementCall {
                    count: s.reinforcement_call.count,
                    ..self.settings.reinforcement_call
                });
            }
            Item::RandomReinforcements => {
                self.set_reinforcement_call(tenchu_core::roster::ReinforcementCall {
                    random: s.reinforcement_call.random,
                    ..self.settings.reinforcement_call
                });
            }
            Item::ReinforcementLimit => self.set_reinforcement_limit(s.reinforcement_limit),
            Item::WitnessedKillReinforcements => {
                self.set_reinforcement_call(tenchu_core::roster::ReinforcementCall {
                    witnessed_kills: false,
                    ..self.settings.reinforcement_call
                });
            }
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
