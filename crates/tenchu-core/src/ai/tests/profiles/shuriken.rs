use super::*;
use crate::ai::profiles::{SHURIKEN_BUDGET, Shuriken};

fn setup() -> (Game, usize) {
    let mut game = oracle::game(0, 0);
    let id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| game.roster.get(id).kind == 0x10)
        .unwrap();
    oracle::flat_ground(&mut game);
    game.teleport_player(Position::new(0, 0, -2000), true)
        .unwrap();
    game.move_actor(id, Position::new(0, 0, 2000), 0, true)
        .unwrap();
    control(&mut game, id, |c| {
        for other in c.roster.order.clone() {
            if other != id && other != c.player.id {
                c.roster.remove(other, c.ctx);
            }
        }
        c.actor.target = Some(c.player.root);
    });
    game.roster.keep_all_active = true;
    game.roster.get_mut(game.player_id).invulnerable = true;
    game.clock = 1000;
    game.set_behavior_profiles(Settings {
        enabled: true,
        combat: Combat::Watchman,
        watchman_shuriken: true,
        ..Default::default()
    });
    ready(&mut game, id);
    (game, id)
}

fn ready(game: &mut Game, id: usize) {
    control(game, id, |c| {
        c.actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2;
        assert!(c.actor.enter(c.ctx, MOT_ENGAGE, true));
        c.actor.motion.count = 0;
        c.actor.attack_clock = c.clock - 1;
        c.state.attack_clock = c.clock - 1;
        c.state.sight = Sight {
            distance: 4000,
            direction: 0,
        };
    });
}

fn begin(game: &mut Game, id: usize) {
    ready(game, id);
    let rng = game.random;
    control(game, id, |c| {
        assert_eq!(
            c.combat_profile(),
            Some(Combat::Watchman),
            "actor {id}: kind {}, source {:?}, think {:?}",
            c.actor.kind,
            c.actor.script_kind,
            c.actor.think
        );
        assert_eq!(c.profile_combat(Combat::Watchman).unwrap(), 0);
        assert_eq!(c.actor.motion.action, MOT_SYURI);
        assert!(c.actor.profile_shuriken.unwrap().pending);
    });
    assert_eq!(game.random, rng, "optional selection consumes no RNG");
}

fn finish_throw(game: &mut Game, id: usize) -> usize {
    let mut ticks = 0;
    while game.roster.get(id).profile_shuriken.unwrap().pending && ticks < 100 {
        game.step(0).unwrap();
        ticks += 1;
    }
    assert!(
        !game.roster.get(id).profile_shuriken.unwrap().pending,
        "throw did not release"
    );
    assert!(
        game.items
            .slots
            .iter()
            .any(|item| { item.kind == crate::items::Kind::Shuriken && item.owner() == Some(id) }),
        "source projectile was not emitted"
    );
    ticks
}

#[test]
#[ignore = "requires TENCHU_DISC; normal AI dispatch selects and releases an optional shot without item input"]
fn watchman_shuriken_runs_through_game_dispatch() {
    let (mut game, id) = setup();
    let mut threw = false;
    for _ in 0..600 {
        game.step(0).unwrap();
        if game
            .roster
            .get(id)
            .profile_shuriken
            .is_some_and(|stock| stock.remaining < SHURIKEN_BUDGET)
        {
            threw = true;
            break;
        }
    }
    assert!(
        threw,
        "Watchman never released a shot through normal AI dispatch"
    );
    assert_eq!(game.roster.get(id).inventory[1], 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; source aim/count/cooldown gates and supported common shuriken motions"]
fn watchman_shuriken_selection_retains_source_gates() {
    let (mut game, id) = setup();
    for (count, degree, distance, ready_clock, status, succeeds) in [
        (0, 99, 4000, true, ActorState::Engage, true),
        (0, -99, 4000, true, ActorState::Engage, true),
        (0, 100, 4000, true, ActorState::Engage, false),
        (0, -100, 4000, true, ActorState::Engage, false),
        (1, 0, 4000, true, ActorState::Engage, false),
        (0, 0, 1, true, ActorState::Engage, false),
        (0, 0, 4000, false, ActorState::Engage, false),
        (0, 0, 4000, true, ActorState::Attack, false),
        (0, 0, 4000, true, ActorState::Chase, false),
        (0, 0, 4000, true, ActorState::Damaged, false),
    ] {
        ready(&mut game, id);
        control(&mut game, id, |c| {
            c.actor.status = status;
            c.actor.motion.count = count;
            c.state.sight = Sight {
                distance,
                direction: degree,
            };
            c.state.attack_clock = c.clock - i32::from(ready_clock);
            let rng = *c.ctx.random;
            assert_eq!(
                c.watchman_shuriken(3000),
                succeeds,
                "{count}/{degree}/{distance}/{ready_clock}/{status:?}"
            );
            assert_eq!(*c.ctx.random, rng);
            assert_eq!(c.actor.inventory[1], 0);
        });
    }
    ready(&mut game, id);
    control(&mut game, id, |c| {
        let registry = &c.ctx.mission.characters[c.actor.definition].motions;
        assert!(!registry.iter().any(|m| m.action == MOT_SYURI));
        assert!(
            c.watchman_shuriken(3000),
            "ordinary guards use the source common clips"
        );
        c.actor.enter(c.ctx, MOT_ENGAGE, true);
        c.actor.motion.count = 0;
        c.state.attack_clock = c.clock - 1;
        c.ctx
            .mission
            .common_motions
            .retain(|m| m.action != MOT_SYURI_RECOVER);
        assert!(
            !c.watchman_shuriken(3000),
            "missing recovery must not enter an unfinishable throw"
        );
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; actual NPC animation releases exactly three optional shots without touching authored stock"]
fn watchman_shuriken_budget_is_finite_and_toggle_safe() {
    let (mut game, id) = setup();
    game.roster.get_mut(id).inventory[1] = 7;
    let mut original_stock = None;
    for shot in 0..SHURIKEN_BUDGET {
        begin(&mut game, id);
        assert_eq!(
            game.roster.get(id).profile_shuriken.unwrap().remaining,
            SHURIKEN_BUDGET - shot
        );
        if shot == 0 {
            game.set_behavior_profiles(Settings {
                enabled: false,
                ..Default::default()
            });
        } else if shot == 1 {
            game.set_behavior_profiles(Settings {
                enabled: true,
                combat: Combat::Original,
                ..Default::default()
            });
        }
        let elapsed = finish_throw(&mut game, id);
        eprintln!(
            "optional shot {} released after {elapsed} source ticks",
            shot + 1
        );
        assert_eq!(game.roster.get(id).inventory[1], 7);
        assert_eq!(
            game.roster.get(id).profile_shuriken.unwrap().remaining,
            SHURIKEN_BUDGET - shot - 1
        );
        original_stock.get_or_insert(game.roster.get(id).inventory);
        game.set_behavior_profiles(Settings {
            enabled: true,
            combat: Combat::Watchman,
            watchman_shuriken: true,
            seed: u16::from(shot) + 10,
            ..Default::default()
        });
    }
    for seed in 0..32 {
        game.set_behavior_profiles(Settings {
            enabled: true,
            combat: Combat::Mixed,
            watchman_shuriken: true,
            seed,
            ..Default::default()
        });
        ready(&mut game, id);
        control(&mut game, id, |c| {
            if let Some(profile) = c.combat_profile() {
                c.profile_combat(profile).unwrap();
            }
        });
        assert_eq!(game.roster.get(id).profile_shuriken.unwrap().remaining, 0);
        assert_eq!(game.roster.get(id).inventory, original_stock.unwrap());
        assert_ne!(game.roster.get(id).motion.action, MOT_SYURI);
    }
    game.set_behavior_profiles(Settings::default());
    ready(&mut game, id);
    control(&mut game, id, |c| {
        c.item().unwrap();
        assert_eq!(
            c.actor.motion.action, MOT_SYURI,
            "Original may still use its real inventory"
        );
    });
    for _ in 0..100 {
        game.step(0).unwrap();
        if game.roster.get(id).inventory[1] < 7 {
            break;
        }
    }
    assert_eq!(game.roster.get(id).inventory[1], 6);
    assert_eq!(game.roster.get(id).profile_shuriken.unwrap().remaining, 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; interrupted windup, saved pending release, and historical stock restoration"]
fn watchman_shuriken_interruption_and_checkpoint_continuation() {
    let (mut game, id) = setup();
    begin(&mut game, id);
    control(&mut game, id, |c| {
        assert!(c.actor.enter(c.ctx, MOT_ENGAGE, true));
        assert_eq!(
            c.actor.profile_shuriken,
            Some(Shuriken {
                remaining: 3,
                pending: false
            })
        );
    });
    begin(&mut game, id);
    game.step(0).unwrap();
    assert!(game.roster.get(id).profile_shuriken.unwrap().pending);
    let checkpoint = serde_json::to_vec(&game).unwrap();
    let mut restored: Game = serde_json::from_slice(&checkpoint).unwrap();
    restored.reuse_scene(&mut game);
    let ticks = finish_throw(&mut game, id);
    assert_eq!(finish_throw(&mut restored, id), ticks);
    assert_eq!(
        serde_json::to_vec(&game).unwrap(),
        serde_json::to_vec(&restored).unwrap()
    );
    let historical: Game = serde_json::from_slice(&checkpoint).unwrap();
    assert_eq!(
        historical
            .roster
            .get(id)
            .profile_shuriken
            .unwrap()
            .remaining,
        3
    );
    assert_eq!(
        restored.roster.get(id).profile_shuriken.unwrap().remaining,
        2
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; optional release matches authored shuriken stock with normal and skipped source draws"]
fn watchman_shuriken_release_matches_source_stock() {
    for skipped_draws in [false, true] {
        let (mut optional, id) = setup();
        begin(&mut optional, id);
        let mut source: Game =
            serde_json::from_slice(&serde_json::to_vec(&optional).unwrap()).unwrap();
        source.roster.get_mut(id).profile_shuriken = None;
        source.roster.get_mut(id).inventory[1] = SHURIKEN_BUDGET;
        let mut ticks = 0;
        while optional.roster.get(id).profile_shuriken.unwrap().pending && ticks < 100 {
            let skipped = skipped_draws && ticks % 3 == 0;
            let cadence = crate::drawing::Cadence {
                step: if skipped {
                    crate::drawing::Step::Skipped
                } else {
                    crate::drawing::Step::Normal
                },
                display_swap: !skipped,
            };
            optional.step_recorded(0, cadence).unwrap();
            source.step_recorded(0, cadence).unwrap();
            ticks += 1;
        }
        assert_eq!(
            optional.roster.get(id).profile_shuriken.unwrap().remaining,
            2
        );
        assert_eq!(source.roster.get(id).inventory[1], 2);
        optional.roster.get_mut(id).profile_shuriken = None;
        optional.roster.get_mut(id).inventory[1] = 2;
        assert_eq!(
            serde_json::to_vec(&optional).unwrap(),
            serde_json::to_vec(&source).unwrap()
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; exclusions, Seeded mix grants, actual reinforcement creation and recycled actor slots"]
fn watchman_shuriken_exclusions_and_recruits() {
    let (mut game, id) = setup();
    let original = game.roster.get(id).clone();
    for kind in [0, 1, 2, 0x80, 0x87, 0x90, 0x98, 0xa0] {
        game.roster.get_mut(id).kind = kind;
        control(&mut game, id, |c| assert!(c.combat_profile().is_none()));
        assert!(game.roster.get(id).profile_shuriken.is_none());
    }
    game.roster.actors[id] = Some(original);
    assert!(!game.intelligence.profiles.applies(
        game.player(),
        &game.mission,
        game.player_id,
        game.intelligence.difficulty
    ));
    game.roster.get_mut(id).scripted = true;
    control(&mut game, id, |c| assert!(c.combat_profile().is_none()));
    game.roster.get_mut(id).scripted = false;
    game.roster.get_mut(id).script_kind = Some(0x80);
    control(&mut game, id, |c| assert!(c.combat_profile().is_none()));
    game.roster.get_mut(id).script_kind = None;

    for profiles in [
        Settings {
            enabled: false,
            watchman_shuriken: true,
            combat: Combat::Watchman,
            ..Default::default()
        },
        Settings {
            enabled: true,
            watchman_shuriken: false,
            combat: Combat::Watchman,
            ..Default::default()
        },
        Settings {
            enabled: true,
            watchman_shuriken: true,
            combat: Combat::Cautious,
            ..Default::default()
        },
    ] {
        game.set_behavior_profiles(profiles);
        ready(&mut game, id);
        control(&mut game, id, |c| {
            if let Some(profile) = c.combat_profile() {
                c.profile_combat(profile).unwrap();
            }
        });
        assert!(game.roster.get(id).profile_shuriken.is_none());
    }
    let seed = (0..100)
        .find(|&seed| {
            Settings {
                combat: Combat::Mixed,
                seed,
                ..Default::default()
            }
            .combat(game.roster.get(id))
                == Combat::Watchman
        })
        .unwrap();
    game.set_behavior_profiles(Settings {
        enabled: true,
        watchman_shuriken: true,
        combat: Combat::Mixed,
        seed,
        ..Default::default()
    });
    begin(&mut game, id);
    assert_eq!(game.roster.get(id).profile_shuriken.unwrap().remaining, 3);

    let mut game = super::super::reinforcement::corner();
    let caller = game.roster.order[1];
    game.set_behavior_profiles(Settings {
        enabled: true,
        watchman_shuriken: true,
        combat: Combat::Watchman,
        ..Default::default()
    });
    let mut previous = None;
    for _ in 0..2 {
        control(&mut game, caller, |c| {
            if let Some(id) = previous {
                c.roster.remove(id, c.ctx);
            }
            c.actor.recruitment = Recruitment::Available;
            c.actor.brain.timer = 2;
            c.actor.brain.chase = [c.actor.position.x, c.actor.position.z];
            c.state.sight = Sight {
                distance: 18000,
                direction: 0,
            };
            c.alarm_with_limits(true).unwrap();
        });
        assert_eq!(game.roster.order.len(), 3);
        let recruit = game.roster.order[2];
        if let Some(previous) = previous {
            assert_eq!(recruit, previous);
        }
        assert_eq!(
            game.roster.get(recruit).recruitment,
            Recruitment::Reinforcement
        );
        assert!(game.roster.get(recruit).profile_shuriken.is_none());
        begin(&mut game, recruit);
        assert_eq!(
            game.roster.get(recruit).profile_shuriken.unwrap().remaining,
            3
        );
        game.roster.get_mut(recruit).profile_shuriken = Some(Shuriken {
            remaining: 0,
            pending: false,
        });
        previous = Some(recruit);
    }
}
