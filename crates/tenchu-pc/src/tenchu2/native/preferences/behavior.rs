use crate::settings::edit::cycle;
use crate::settings::{Settings, menu::Item};
use tenchu_core::ai::profiles::{Combat, Idle};

pub(super) fn change(settings: &mut Settings, item: Item, delta: i32, reset: bool) -> bool {
    let defaults = Settings::default();
    let profile = &mut settings.behavior_profiles;
    let on = |value: bool| !reset && !value;
    match item {
        Item::BehaviorProfiles => profile.enabled = on(profile.enabled),
        Item::BehaviorCombat => {
            profile.combat = if reset {
                defaults.behavior_profiles.combat
            } else {
                cycle(profile.combat, &Combat::ALL, delta)
            }
        }
        Item::BehaviorIdle => {
            profile.idle = if reset {
                defaults.behavior_profiles.idle
            } else {
                cycle(profile.idle, &Idle::ALL, delta)
            }
        }
        Item::BehaviorProgression => profile.progression = on(profile.progression),
        Item::BehaviorSeed => {
            profile.seed = if reset {
                defaults.behavior_profiles.seed
            } else {
                (i32::from(profile.seed) + delta).clamp(0, 65535) as u16
            }
        }
        Item::ReinforcementsPerCall => {
            settings.reinforcement_call.count = if reset {
                defaults.reinforcement_call.count
            } else {
                (i32::from(settings.reinforcement_call.count) + delta).clamp(
                    0,
                    i32::from(tenchu_core::roster::ReinforcementCall::MAX_COUNT),
                ) as u16
            }
        }
        Item::ReinforcementLimit => {
            settings.reinforcement_limit = if reset {
                defaults.reinforcement_limit
            } else {
                (i32::from(settings.reinforcement_limit) + delta * 10).clamp(0, 1000) as u16
            }
        }
        Item::RandomReinforcements => {
            settings.reinforcement_call.random = on(settings.reinforcement_call.random)
        }
        Item::WitnessedKillReinforcements => {
            settings.reinforcement_call.witnessed_kills =
                on(settings.reinforcement_call.witnessed_kills)
        }
        _ => return false,
    }
    true
}
