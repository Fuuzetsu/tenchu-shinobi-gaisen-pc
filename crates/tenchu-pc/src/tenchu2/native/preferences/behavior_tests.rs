use super::{
    gameplay_tests::{playback, session},
    *,
};
use crate::tenchu2::native::campaign::Content;
use tenchu_core::ai::profiles::{Combat, Idle};

fn apply(menu: &mut Menu, settings: &mut Settings, edit: Edit) {
    menu.apply(
        edit,
        &mut State {
            settings,
            paused: &mut true,
            controls: &mut Controls::default(),
            presets: &mut Manager::new(
                std::env::temp_dir().join("tenchu-native-profile-presets.json"),
            ),
            lights: 0,
            samples: &[1, 4],
            models: None,
        },
        &mut Action::default(),
    );
    assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; actual native F1 edits/reset and per-guard overrides on both stage types"]
fn native_behavior_menu_applies_settings_and_guard_overrides() {
    for imported in [false, true] {
        let mut p = playback(imported);
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        settings.set_visual_preset(settings::presets::MODERN_ULTRA);
        let graphics = settings.graphics.clone();
        for (item, delta) in [
            (Item::BehaviorProfiles, 1),
            (Item::BehaviorCombat, 2),
            (Item::BehaviorIdle, 2),
            (Item::BehaviorSeed, 41),
            (Item::ReinforcementsPerCall, 2),
            (Item::RandomReinforcements, 1),
            (Item::WitnessedKillReinforcements, 1),
            (Item::ReinforcementLimit, 7),
        ] {
            apply(&mut menu, &mut settings, Edit::Change(item, delta));
        }
        p.configure(&settings).unwrap();
        let w = &session(&p.content).world;
        assert!(w.ai.profiles.enabled);
        assert_eq!(w.ai.profiles.seed, 42);
        assert_eq!(w.ai.profiles.combat, Combat::Coward);
        assert_eq!(w.ai.profiles.idle, Idle::Lookout);
        assert_eq!(w.ai.reinforcements.settings.count, 3);
        assert!(
            w.ai.reinforcements.settings.random && w.ai.reinforcements.settings.witnessed_kills
        );
        assert_eq!(w.ai.reinforcements.limit, Some(70));
        assert_eq!(settings.graphics, graphics);
        let guard = (0..w.actors.len())
            .find(|&id| w.profile_eligible(id))
            .unwrap();
        menu.tools.sync(&p.content, p.generation);
        menu.tools.debug.selected_actor = guard;
        menu.tools.sync(&p.content, p.generation);
        assert!(menu.tools.debug.actor_behavior.is_some());
        apply(&mut menu, &mut settings, Edit::Change(Item::ActorCombat, 3));
        apply(&mut menu, &mut settings, Edit::Change(Item::ActorIdle, 4));
        menu.apply_tools(&mut p.content, [800, 600]);
        assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
        let selected = session(&p.content).world.profile_selection(guard).unwrap();
        assert_eq!(selected.combat, Combat::Reckless);
        assert_eq!(selected.idle, Idle::TracePlayer);
        for item in [Item::ActorCombat, Item::ActorIdle] {
            apply(&mut menu, &mut settings, Edit::Reset(item));
        }
        menu.apply_tools(&mut p.content, [800, 600]);
        let selected = session(&p.content).world.profile_selection(guard).unwrap();
        assert_eq!(selected.combat, settings.behavior_profiles.combat);
        assert_eq!(selected.idle, settings.behavior_profiles.idle);
        apply(
            &mut menu,
            &mut settings,
            Edit::Change(Item::BehaviorProgression, 1),
        );
        p.configure(&settings).unwrap();
        assert!(session(&p.content).world.profile_selection(guard).is_none());
        for item in [
            Item::BehaviorProfiles,
            Item::BehaviorCombat,
            Item::BehaviorIdle,
            Item::BehaviorSeed,
            Item::BehaviorProgression,
            Item::ReinforcementsPerCall,
            Item::RandomReinforcements,
            Item::WitnessedKillReinforcements,
            Item::ReinforcementLimit,
        ] {
            apply(&mut menu, &mut settings, Edit::Reset(item));
        }
        p.configure(&settings).unwrap();
        assert_eq!(settings.behavior_profiles, Default::default());
        assert_eq!(settings.reinforcement_call, Default::default());
        assert_eq!(settings.reinforcement_limit, 0);
        assert_eq!(settings.graphics, graphics);
        menu.tools.sync(&p.content, p.generation);
        assert!(menu.tools.debug.actor_behavior.is_none());
        let Content::Gameplay(s) = &p.content else {
            unreachable!()
        };
        assert!(
            s.world
                .actors
                .iter()
                .all(|a| a.behavior.profile.selection.is_none())
        );
    }
}
