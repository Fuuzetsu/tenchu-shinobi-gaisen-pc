use super::*;
use tenchu_core::{actor::State, game::events::Outcome, ids::MOT_DEAD, math::Position};

fn kill(game: &mut Game) {
    let player = game.roster.get_mut(game.player_id);
    player.health = 0;
    player.status = State::Dead;
    player.motion.change(
        MOT_DEAD,
        &game.mission.characters[player.definition].motions,
        &game.mission.common_motions,
        &game.mission.clips,
    );
}

fn flames(game: &Game) -> Vec<Position> {
    game.misc
        .objects
        .iter()
        .filter(|o| {
            !o.paused
                && matches!(
                    o.kind,
                    tenchu_assets::misc::Kind::Sprite(_) | tenchu_assets::misc::Kind::Bonfire(_)
                )
                && (0..3)
                    .all(|axis| (o.position[axis] - game.player().position[axis]).abs() < 10000)
        })
        .map(|o| o.position)
        .collect()
}

fn assert_flames(game: &Game, positions: &[Position]) {
    for &position in positions {
        assert!(
            game.effects
                .billboards
                .iter()
                .any(|b| b.position == position),
            "Mission {} lost flame {position:?} at tick {}",
            game.mission.stage.order,
            game.tick
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; boss failure cutscenes must hold before effects are cleared and preserve retry progress"]
fn boss_death_cutscene_keeps_the_world_alive_until_confirmation() {
    for character in 0..2 {
        for live in [false, true] {
            let options = Options {
                mission: 1,
                character,
                ..Default::default()
            };
            let mut app = application(options);
            let mut reference = application(options);
            app.settings.wait_after_death = true;
            app.settings.live_death_scene = live;
            app.settings.frontend.auto_save = false;
            for application in [&mut app, &mut reference] {
                assert!(application.game_tick(0));
                application
                    .campaign
                    .game
                    .teleport_player(Position::new(1000, 0, 5000), false)
                    .unwrap();
                assert!(application.game_tick(0));
                assert!(application.campaign.game.cinematic.active);
                for _ in 0..5000 {
                    if !application.campaign.game.cinematic.active {
                        break;
                    }
                    assert!(application.game_tick(0));
                }
                assert!(!application.campaign.game.cinematic.active);
                assert!(application.game_tick(0));
                kill(&mut application.campaign.game);
            }
            for _ in 0..5000 {
                reference.campaign.tick(0).unwrap();
                if !app.game_tick(0) {
                    break;
                }
            }
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            assert_eq!(app.campaign.game.cinematic.sequence, 250);
            assert_eq!(app.campaign.game.death_wait_ticks(), Some(1));
            assert_eq!(app.campaign.game.events.outcome, Outcome::Running);
            assert!(
                crate::session::tests::state(&app.campaign.game)
                    == crate::session::tests::state(&reference.campaign.game)
            );
            let particles = app.campaign.game.effects.billboards.len();
            assert!(particles > 0, "Death cutscene effects disappeared on entry");
            assert_eq!(particles, reference.campaign.game.effects.billboards.len());
            let tick = app.campaign.game.tick;
            for _ in 0..600 {
                assert_eq!(app.game_tick(0), live);
                if !live {
                    assert_eq!(app.campaign.game.effects.billboards.len(), particles);
                }
            }
            if live {
                let game = &app.campaign.game;
                assert!(game.scene.models[game.player().parts[0]].conflict.is_none());
            }
            assert_eq!(app.campaign.game.tick, tick + if live { 600 } else { 0 });
            assert_eq!(app.campaign.game.death_wait_ticks(), Some(1));
            reference.campaign.tick(0).unwrap();
            assert!(matches!(reference.campaign.screen, Screen::GameOver));
            app.action(Action::Start);
            assert!(matches!(app.campaign.screen, Screen::GameOver));
            assert_eq!(app.campaign.seed, reference.campaign.seed);
            assert_eq!(app.campaign.progress, reference.campaign.progress);
            if !live {
                assert!(
                    crate::session::tests::state(&app.campaign.game)
                        == crate::session::tests::state(&reference.campaign.game)
                );
            }
            for application in [&mut app, &mut reference] {
                application.action(Action::Confirm);
                assert!(matches!(application.campaign.screen, Screen::Inventory(_)));
                application.action(Action::Confirm);
                assert!(application.campaign.playing());
            }
            assert!(
                crate::session::tests::state(&app.campaign.game)
                    == crate::session::tests::state(&reference.campaign.game)
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; track actual flame emitters through ordinary deaths in every mission"]
fn ordinary_death_observation_retains_nearby_flames() {
    let mut checked = 0;
    for mission in 1..=10 {
        let mut app = application(Options {
            character: 1,
            mission,
            ..Default::default()
        });
        app.settings.wait_after_death = true;
        app.settings.live_death_scene = true;
        let events = std::mem::take(&mut app.campaign.game.events);
        for _ in 0..30 {
            app.campaign.game.step(0).unwrap();
        }
        app.campaign.game.events = events;
        kill(&mut app.campaign.game);
        for _ in 0..5000 {
            if !app.game_tick(0) {
                break;
            }
        }
        assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
        let flames = flames(&app.campaign.game);
        checked += flames.len();
        for _ in 0..600 {
            assert!(app.game_tick(0));
            assert_flames(&app.campaign.game, &flames);
        }
    }
    assert!(checked > 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; an overlapping guard must not shove the player corpse during observation"]
fn death_observation_removes_player_contacts_without_creating_an_enemy_pool() {
    for character in 0..2 {
        let mut app = application(Options {
            character,
            ..Default::default()
        });
        app.settings.wait_after_death = true;
        app.settings.live_death_scene = true;
        app.settings.frontend.auto_save = false;
        app.campaign.game.roster.keep_all_active = true;
        let events = std::mem::take(&mut app.campaign.game.events);
        for _ in 0..30 {
            app.campaign.game.step(0).unwrap();
        }
        app.campaign.game.events = events;
        kill(&mut app.campaign.game);
        for _ in 0..3000 {
            if !app.game_tick(0) {
                break;
            }
        }
        assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
        let game = &mut app.campaign.game;
        let position = game.player().position;
        let rotation = game.player().rotation;
        let body = game.player().parts[0];
        assert!(game.scene.models[body].conflict.is_some());
        let guard = *game
            .roster
            .order
            .iter()
            .find(|&&id| {
                let actor = game.roster.get(id);
                id != game.player_id
                    && actor.health > 0
                    && game.scene.models[actor.parts[0]].conflict.is_some()
                    && game.scene.models[actor.parts[0]].attribute & tenchu_core::scene::COLLIDE
                        != 0
            })
            .unwrap();
        game.collision.compute(&mut game.scene).unwrap();
        let center =
            |body: usize| game.collision.slots[game.scene.models[body].conflict.unwrap()].position;
        let player_center = center(body);
        let guard_center = center(game.roster.get(guard).parts[0]);
        let actor = game.roster.get_mut(guard);
        for axis in 0..3 {
            actor.position[axis] += player_center[axis] - guard_center[axis];
        }
        game.scene.update(actor.root_model, actor.root(&game.math));
        game.scene.finish_tick();
        game.collision.compute(&mut game.scene).unwrap();
        assert!(game.collision.result(&game.scene, body, None).is_some());
        for _ in 0..120 {
            assert!(app.game_tick(0));
            let game = &app.campaign.game;
            assert!(game.scene.models[body].conflict.is_none());
            assert_eq!(game.player().position, position);
            assert_eq!(game.player().rotation, rotation);
            assert!(game.effects.blood_pools.is_empty());
        }
    }
}
