use super::*;
use crate::{
    ai::profiles::{Combat, Idle, Overrides, Settings},
    game::Game,
};

fn guard(game: &Game) -> usize {
    *game
        .roster
        .order
        .iter()
        .find(|&&id| {
            crate::ai::profiles::eligible(game.roster.get(id), &game.mission, game.player_id)
        })
        .unwrap()
}

fn control<T>(game: &mut Game, id: usize, run: impl FnOnce(&mut Controller<'_, '_>) -> T) -> T {
    let mut actor = game.roster.take(id);
    let mut ctx = Context {
        mission: &mut game.mission,
        math: &game.math,
        body: &game.body,
        cursor: &mut game.cursor,
        random: &mut game.random,
        effects: &mut game.effects,
        scene: &mut game.scene,
        collision: &mut game.collision,
        camera: &mut game.camera,
        drawing: &mut game.drawing,
    };
    let player = Player::new(game.roster.get(game.player_id), &ctx);
    let mut c = Controller::new(
        &mut actor,
        &mut game.roster,
        &mut game.items,
        &mut game.intelligence,
        Input {
            player,
            clock: game.clock,
            buttons: 0,
            commands: &game.commands,
        },
        &mut ctx,
    );
    let result = run(&mut c);
    game.roster.restore(actor);
    result
}

#[test]
#[ignore = "requires TENCHU_DISC; stable assignment, per-guard overrides, progression and story protection"]
fn behavior_profiles_assignment_and_boundaries() {
    let mut game = oracle::game(0, 0);
    let id = guard(&game);
    let rng = game.random;
    let mut settings = Settings {
        enabled: true,
        ..Default::default()
    };
    let actor = game.roster.get(id);
    let assignments: Vec<_> = (0..32)
        .map(|seed| Settings { seed, ..settings }.combat(actor))
        .collect();
    assert!(Combat::ALL[1..6].iter().all(|p| assignments.contains(p)));
    for (seed, &expected) in assignments.iter().enumerate() {
        assert_eq!(
            Settings {
                seed: seed as u16,
                ..settings
            }
            .combat(actor),
            expected
        );
    }
    assert_eq!(game.random, rng);
    game.set_actor_behavior(
        id,
        Overrides {
            combat: Some(Combat::Cautious),
            idle: Some(Idle::Lookout),
        },
    )
    .unwrap();
    assert_eq!(settings.combat(game.roster.get(id)), Combat::Cautious);
    assert!(!game.intelligence.profiles.enabled);
    assert!(control(&mut game, id, |c| c.combat_profile()).is_none());
    assert!(
        game.set_actor_behavior(game.player_id, Default::default())
            .is_err()
    );
    let original = game.roster.get(id).clone();
    for kind in [0x80, 0x87, 0x91, 0xa0] {
        game.roster.get_mut(id).kind = kind;
        assert!(game.set_actor_behavior(id, Default::default()).is_err());
    }
    game.roster.actors[id] = Some(original.clone());
    game.roster.get_mut(id).script_kind = Some(0x80);
    assert!(game.set_actor_behavior(id, Default::default()).is_err());
    game.roster.actors[id] = Some(original);
    game.cinematic.active = true;
    assert!(game.set_actor_behavior(id, Default::default()).is_err());
    game.cinematic.active = false;
    settings.progression = true;
    for (difficulty, threshold) in [
        (Difficulty::Easy, 6),
        (Difficulty::Normal, 4),
        (Difficulty::Hard, 2),
    ] {
        for order in 1..=10 {
            game.mission.stage.order = order;
            assert_eq!(
                settings.applies(
                    game.roster.get(id),
                    &game.mission,
                    game.player_id,
                    difficulty
                ),
                order >= threshold
            );
        }
    }
    let brain = game.roster.get(id).brain.clone();
    game.set_behavior_profiles(Settings {
        seed: 900,
        combat: Combat::Flanker,
        ..Default::default()
    });
    assert_eq!(
        game.roster.get(id).brain,
        brain,
        "Editing disabled profiles must not reset authored AI"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; profile decisions preserve committed motions, patrol route state and original sleepers"]
fn behavior_profiles_decisions_and_idle_routes() {
    let mut game = oracle::game(0, 0);
    let id = guard(&game);
    game.set_behavior_profiles(Settings {
        enabled: true,
        combat: Combat::Reckless,
        ..Default::default()
    });
    let a = game.roster.get_mut(id);
    a.status = ActorState::Engage;
    a.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2;
    a.inventory[3] = 3;
    a.health = 1;
    game.clock = 1000;
    game.intelligence.sight = Sight {
        distance: 1000,
        direction: 0,
    };
    control(&mut game, id, |c| {
        assert_ne!(c.combat_program().unwrap() & SQUARE, 0);
        c.player.status = ActorState::Attack;
        assert_eq!(c.profile_combat(Combat::Cautious).unwrap(), DOWN);
        assert_eq!(c.profile_combat(Combat::Watchman).unwrap(), DOWN);
        let motion = c.actor.motion.clone();
        c.item().unwrap();
        assert_eq!(c.actor.motion.action, motion.action);
        assert_eq!(c.actor.inventory[3], 3);
        for status in [
            ActorState::Damaged,
            ActorState::Transition,
            ActorState::Jump,
            ActorState::Dead,
            ActorState::UseItem,
        ] {
            c.actor.status = status;
            for profile in Combat::ALL[1..6].iter().copied() {
                assert_eq!(c.profile_combat(profile).unwrap(), 0);
                assert_eq!(c.actor.motion.action, motion.action);
            }
        }
        c.actor.status = ActorState::Normal;
        c.actor.think.idle = programs::Idle::Trace;
        c.state.profiles.idle = Idle::Patrol;
        let at = c.actor.position;
        c.actor.patrol = Some(crate::actor::patrol::Patrol::new(&[(at.to_array(), 0)], at));
        c.state.profiles.quick_turns = true;
        assert_eq!(c.profile_idle().unwrap(), Some(0xf000));
        assert_eq!(c.actor.patrol.as_ref().unwrap().index, 0);
        c.state.profiles.quick_turns = false;
        assert_eq!(c.profile_idle().unwrap(), Some(0));
        c.state.profiles.idle = Idle::Lookout;
        assert_eq!(c.profile_idle().unwrap().unwrap() & (UP | DOWN), 0);
        c.state.profiles.idle = Idle::TracePlayer;
        c.actor.brain = Default::default();
        c.state.sight.direction = 900;
        assert_eq!(c.profile_idle().unwrap(), Some(RIGHT));
        c.actor.think.idle = programs::Idle::Sleep;
        assert_eq!(c.profile_idle().unwrap(), None);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; real guard attacks still damage the player under each offensive behavior"]
fn behavior_profiles_combat_can_hit() {
    for ranged in [false, true] {
        for profile in [
            Combat::Reckless,
            Combat::Cautious,
            Combat::Flanker,
            Combat::Watchman,
        ] {
            let mut game = oracle::game(0, 0);
            let id = *game
                .roster
                .order
                .iter()
                .find(|&&id| {
                    let actor = game.roster.get(id);
                    crate::ai::profiles::eligible(actor, &game.mission, game.player_id)
                        && if ranged {
                            game.mission.characters[actor.definition].weapon >> 4 == 3
                        } else {
                            actor.kind == 0x10
                        }
                })
                .expect("Mission 1 needs both a sword guard and an archer");
            oracle::flat_ground(&mut game);
            game.teleport_player(Position::ZERO, true).unwrap();
            game.move_actor(id, Position::new(0, 0, 2800), 0, true)
                .unwrap();
            control(&mut game, id, |c| {
                for other in c.roster.order.clone() {
                    if other != id && other != c.player.id {
                        c.roster.remove(other, c.ctx);
                    }
                }
            });
            game.roster.get_mut(game.player_id).health = 30000;
            game.roster.get_mut(game.player_id).max_health = 30000;
            game.roster.get_mut(game.player_id).rotation[1] = 0;
            game.intelligence.difficulty = Difficulty::Hard;
            game.set_behavior_profiles(Settings {
                enabled: true,
                combat: profile,
                ..Default::default()
            });
            let mut attacks = 0;
            for _ in 0..900 {
                game.step(0).unwrap();
                let a = game.roster.get(id);
                attacks += usize::from(a.status == ActorState::Attack);
            }
            eprintln!(
                "{profile:?}, ranged={ranged}: attacks={attacks}, hp={}, state={:?}, pos={:?}, phase={}",
                game.player().health,
                game.roster.get(id).status,
                game.roster.get(id).position,
                game.roster.get(id).attribute & ATTR_PHASE
            );
            assert!(attacks > 0, "{profile:?} never attacked");
            assert!(
                game.player().health < 30000,
                "{profile:?} attacks failed to connect"
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; coward recruits require cover/capacity and cannot recursively call even with ordinary activation"]
fn behavior_profiles_reinforcements_are_bounded() {
    for all_active in [false, true] {
        let mut game = super::reinforcement::corner();
        let id = guard(&game);
        game.roster.keep_all_active = all_active;
        game.set_behavior_profiles(Settings {
            enabled: true,
            combat: Combat::Original,
            ..Default::default()
        });
        game.set_actor_behavior(
            id,
            Overrides {
                combat: Some(Combat::Coward),
                idle: None,
            },
        )
        .unwrap();
        game.roster.reinforcement_call.count = 3;
        game.intelligence.sight = Sight {
            distance: 18000,
            direction: 0,
        };
        control(&mut game, id, |c| {
            c.profile_combat(Combat::Coward).unwrap();
        });
        assert_eq!(game.roster.order.len(), 5);
        assert_eq!(game.roster.get(id).recruitment, Recruitment::Called);
        for actor in game.roster.order.clone().into_iter().skip(1) {
            for _ in 0..5 {
                control(&mut game, actor, |c| {
                    c.actor.status = ActorState::Engage;
                    c.actor.brain.timer = 2;
                    c.state.sight.distance = 18000;
                    c.alarm().unwrap();
                    c.profile_combat(Combat::Coward).unwrap();
                });
                assert_eq!(game.roster.order.len(), 5);
            }
        }
        let caller = game.roster.get_mut(id);
        caller.recruitment = Recruitment::Available;
        caller.brain.timer = 2;
        oracle::flat_ground(&mut game);
        control(&mut game, id, |c| {
            c.profile_combat(Combat::Coward).unwrap();
        });
        assert_eq!(game.roster.order.len(), 5, "No spawning in open view");
        game.roster.reinforcement_call.count = 0;
        control(&mut game, id, |c| assert!(!c.can_recruit_with_limits(true)));
    }
}
