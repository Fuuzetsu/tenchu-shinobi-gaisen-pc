use super::*;
use tenchu_core::{
    ai::profiles::{Combat, Idle},
    input,
};

#[test]
#[ignore = "requires TENCHU_DISC; source throw continuation through file save, rewind, and reuse of retained assets"]
fn watchman_shuriken_file_and_rewind_keep_pending_and_spent_stock() {
    use tenchu_core::{
        actor::State,
        ai::profiles::{Settings as Profiles, Shuriken},
        ids,
    };
    let mut app = application(Options::default());
    app.campaign.game.events = Default::default();
    app.set_behavior_profiles(Profiles {
        enabled: true,
        combat: Combat::Watchman,
        watchman_shuriken: true,
        ..Default::default()
    });
    app.keep_enemies_active(true);
    let game = &mut app.campaign.game;
    let id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| game.roster.get(id).kind == 0x10)
        .unwrap();
    let actor = game.roster.get_mut(id);
    actor.motion.change(
        ids::MOT_SYURI,
        &game.mission.characters[actor.definition].motions,
        &game.mission.common_motions,
        &game.mission.clips,
    );
    actor.status = State::AimShuriken;
    actor.profile_shuriken = Some(Shuriken {
        remaining: 3,
        pending: true,
    });
    actor.inventory[1] = 7;
    let saved = app.capture_state().unwrap();
    let rewind = app.capture_rewind().unwrap();
    let pending = postcard::to_allocvec(&app.campaign.game).unwrap();
    for _ in 0..20 {
        app.campaign.game.step(0).unwrap();
    }
    assert_eq!(
        app.campaign.game.roster.get(id).profile_shuriken,
        Some(Shuriken {
            remaining: 2,
            pending: false
        })
    );
    assert_eq!(app.campaign.game.roster.get(id).inventory[1], 7);
    let completed = postcard::to_allocvec(&app.campaign.game).unwrap();
    for file in [true, false] {
        app.reset_item(Item::BehaviorShuriken).unwrap();
        let retained_map = app.campaign.game.mission.map.clone();
        if file {
            app.restore_state(&saved).unwrap();
        } else {
            app.restore_rewind(&rewind).unwrap();
        }
        assert!(app.settings.behavior_profiles.watchman_shuriken);
        assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), pending);
        if !file {
            assert!(std::sync::Arc::ptr_eq(
                &retained_map,
                &app.campaign.game.mission.map
            ));
        }
        for _ in 0..20 {
            app.campaign.game.step(0).unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&app.campaign.game).unwrap(),
            completed
        );
    }
    app.campaign.game.roster.get_mut(id).profile_shuriken = Some(Shuriken {
        remaining: 0,
        pending: false,
    });
    let exhausted = app.capture_state().unwrap();
    app.restore_state(&saved).unwrap();
    app.restore_state(&exhausted).unwrap();
    app.apply_item(Item::BehaviorSeed, 200).unwrap();
    app.reset_item(Item::BehaviorShuriken).unwrap();
    app.apply_item(Item::BehaviorShuriken, 1).unwrap();
    assert_eq!(
        app.campaign
            .game
            .roster
            .get(id)
            .profile_shuriken
            .unwrap()
            .remaining,
        0
    );
    assert!(
        crate::settings::runtime::Runtime::Tenchu2
            .unavailable(Item::BehaviorShuriken)
            .is_some()
    );
    for (owner, remaining, pending) in [
        (id, 4, false),
        (id, 0, true),
        (app.campaign.game.player_id, 3, false),
    ] {
        app.campaign.game.roster.get_mut(owner).profile_shuriken =
            Some(Shuriken { remaining, pending });
        let invalid = app.capture_state().unwrap();
        assert!(
            app.restore_state(&invalid)
                .unwrap_err()
                .contains("Invalid Watchman shuriken stock")
        );
        app.restore_state(&saved).unwrap();
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; disabled profile tuning preserves exact game/RNG state and enabled per-actor choices survive save/rewind"]
fn behavior_profiles_parity_save_and_rewind() {
    let mut app = application(Options::default());
    let mut original = application(Options::default());
    app.campaign.game.events = Default::default();
    original.campaign.game.events = Default::default();
    app.apply_item(Item::BehaviorSeed, 1234).unwrap();
    app.apply_item(Item::BehaviorIdle, 1).unwrap();
    app.apply_item(Item::BehaviorShuriken, 1).unwrap();
    for tick in 0..240 {
        let buttons = if tick < 60 {
            input::UP
        } else if tick % 60 == 0 {
            input::SQUARE
        } else {
            0
        };
        app.game_tick(buttons);
        original.game_tick(buttons);
        let settings = app.campaign.game.intelligence.profiles;
        app.campaign.game.intelligence.profiles = Default::default();
        assert!(
            postcard::to_allocvec(&app.campaign.game).unwrap()
                == postcard::to_allocvec(&original.campaign.game).unwrap(),
            "Disabled profiles changed tick {tick}"
        );
        app.campaign.game.intelligence.profiles = settings;
    }
    app.apply_item(Item::BehaviorProfiles, 1).unwrap();
    let game = &app.campaign.game;
    let guard = *game
        .roster
        .order
        .iter()
        .find(|&&id| {
            tenchu_core::ai::profiles::eligible(game.roster.get(id), &game.mission, game.player_id)
        })
        .unwrap();
    app.debug.selected_actor = guard;
    app.apply_item(Item::ActorCombat, 3).unwrap();
    app.apply_item(Item::ActorIdle, 3).unwrap();
    let actor = app.campaign.game.roster.get(guard);
    assert_eq!(actor.behavior.combat, Some(Combat::Reckless));
    assert_eq!(actor.behavior.idle, Some(Idle::Lookout));
    assert!(app.debug.actors_edited && !app.trial.eligible);
    let saved = app.capture_state().unwrap();
    let mut restored = application(Options::default());
    restored.restore_state(&saved).unwrap();
    assert_eq!(
        app.settings.behavior_profiles,
        restored.settings.behavior_profiles
    );
    assert_eq!(
        actor.behavior,
        restored.campaign.game.roster.get(guard).behavior
    );
    for _ in 0..150 {
        app.game_tick(0);
        restored.game_tick(0);
    }
    assert!(!app.faulted && !restored.faulted);
    assert!(
        postcard::to_allocvec(&app.campaign.game).unwrap()
            == postcard::to_allocvec(&restored.campaign.game).unwrap()
    );
    app.settings.rewind_seconds = 5;
    for _ in 0..4 {
        app.game_tick(0);
    }
    let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
    let index = app.rewind.entries.len() - 1;
    app.apply_item(Item::BehaviorSeed, 333).unwrap();
    app.apply_item(Item::ActorCombat, 1).unwrap();
    for _ in 0..45 {
        app.game_tick(0);
    }
    app.rewind_to(index).unwrap();
    assert!(postcard::to_allocvec(&app.campaign.game).unwrap() == expected);
    assert_eq!(
        app.settings.behavior_profiles,
        restored.settings.behavior_profiles
    );
    app.debug.selected_actor = guard;
    app.reset_item(Item::ActorCombat).unwrap();
    app.reset_item(Item::ActorIdle).unwrap();
    assert_eq!(
        app.campaign.game.roster.get(guard).behavior,
        Default::default()
    );
    app.debug.selected_actor = app.campaign.game.player_id;
    assert!(app.apply_item(Item::ActorCombat, 1).is_err());
}
