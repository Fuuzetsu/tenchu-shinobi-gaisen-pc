use super::*;
use crate::settings::edit::cycle as choose;
use tenchu_core::ai::profiles::{Combat, Idle, Settings as Profiles};

impl Application {
    pub(super) fn set_behavior_profiles(&mut self, settings: Profiles) {
        if self.campaign.game.intelligence.profiles != settings {
            self.stop_edited_recording();
        }
        self.settings.behavior_profiles = settings;
        self.campaign.game.set_behavior_profiles(settings);
        self.debug.update(&self.campaign.game);
    }

    pub(super) fn edit_behavior(&mut self, item: Item, direction: i32, reset: bool) -> Result<()> {
        if matches!(item, Item::ActorCombat | Item::ActorIdle) {
            if !self.campaign.playing() || self.demo.is_some() || self.benchmarks.run.is_some() {
                return Err("Start a mission before editing guard behavior".into());
            }
            self.debug.update(&self.campaign.game);
            let mut overrides = self.debug.actor_behavior.ok_or("Select an ordinary living guard; story actors, bosses, civilians and animals are protected")?;
            match item {
                Item::ActorCombat => {
                    overrides.combat = if reset {
                        None
                    } else {
                        choose(
                            overrides.combat,
                            &[None]
                                .into_iter()
                                .chain(Combat::ALL.map(Some))
                                .collect::<Vec<_>>(),
                            direction,
                        )
                    }
                }
                _ => {
                    overrides.idle = if reset {
                        None
                    } else {
                        choose(
                            overrides.idle,
                            &[None]
                                .into_iter()
                                .chain(Idle::ALL.map(Some))
                                .collect::<Vec<_>>(),
                            direction,
                        )
                    }
                }
            }
            self.campaign
                .game
                .set_actor_behavior(self.debug.selected_actor, overrides)?;
            self.stop_edited_recording();
            self.debug.actors_edited = true;
            self.trial.eligible = false;
            self.debug.update(&self.campaign.game);
            return Ok(());
        }
        let mut settings = self.settings.behavior_profiles;
        let defaults = Profiles::default();
        match item {
            Item::BehaviorProfiles => settings.enabled = !reset && !settings.enabled,
            Item::BehaviorCombat => {
                settings.combat = if reset {
                    defaults.combat
                } else {
                    choose(settings.combat, &Combat::ALL, direction)
                }
            }
            Item::BehaviorIdle => {
                settings.idle = if reset {
                    defaults.idle
                } else {
                    choose(settings.idle, &Idle::ALL, direction)
                }
            }
            Item::BehaviorQuickTurns => settings.quick_turns = reset || !settings.quick_turns,
            Item::BehaviorProgression => settings.progression = !reset && !settings.progression,
            Item::BehaviorSeed => {
                settings.seed = if reset {
                    defaults.seed
                } else {
                    (i32::from(settings.seed) + direction).clamp(0, 65535) as u16
                }
            }
            _ => unreachable!(),
        }
        self.set_behavior_profiles(settings);
        Ok(())
    }
}
