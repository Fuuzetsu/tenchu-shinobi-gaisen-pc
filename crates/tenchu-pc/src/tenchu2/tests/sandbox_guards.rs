use super::*;
use tenchu_core::{
    ai::Difficulty,
    ids::{ATTR_PHASE, ATTR_WEAPON_DRAWN},
    input,
};

fn assets() -> GameAssets<DiscSource> {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap()
}

fn sandbox() -> Sandbox {
    let path = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    Sandbox::open(path.as_ref(), None, Some(4), Some(0x10)).unwrap()
}

#[test]
#[ignore = "requires both discs; compares sandbox health with the selected Gaisen controller"]
fn sandbox_guard_health_matches_controller() {
    let assets = assets();
    let definitions = Executable::new(&assets.executable)
        .unwrap()
        .characters()
        .unwrap();
    let mut sandbox = sandbox();
    for kind in [0x10, 0x11, 0x12] {
        sandbox.guard = Some(kind);
        let expected = definitions.iter().find(|c| c.kind == kind).unwrap().health;
        let mut actors = 0;
        for source_models in [false, true] {
            sandbox.source_models = source_models;
            for character in 0..2 {
                for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
                    let game = sandbox
                        .load(
                            &assets,
                            Options {
                                character,
                                difficulty,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    for &id in &game.roster.order {
                        if id == game.player_id {
                            continue;
                        }
                        let actor = game.roster.get(id);
                        assert_eq!((actor.health, actor.max_health), (expected, expected));
                        assert_eq!(game.mission.characters[actor.definition].health, expected);
                        actors += 1;
                    }
                }
            }
        }
        println!("guard {kind:#x}: {expected} HP, {actors} loaded actors checked");
    }
}

fn lookout(
    sandbox: &mut Sandbox,
    assets: &GameAssets<DiscSource>,
    all_active: bool,
    side: i32,
) -> (Game, usize) {
    sandbox.layout.as_mut().unwrap().start = [side * if all_active { 30000 } else { 16500 }, 0, 0];
    let mut game = sandbox.load(assets, Options::default()).unwrap();
    assert_eq!(game.roster.order.len(), 2);
    let id = *game
        .roster
        .order
        .iter()
        .find(|&&id| id != game.player_id)
        .unwrap();
    assert!(game.roster.get(id).patrol.is_none());
    game.roster.keep_all_active = all_active;
    (game, id)
}

#[test]
#[ignore = "requires both discs; unseen stationary guards must not track the player's bearing"]
fn sandbox_lookouts_do_not_follow_unseen_player() {
    let assets = assets();
    let mut sandbox = stationary_arena();
    for kind in [0x10, 0x11, 0x12] {
        sandbox.guard = Some(kind);
        for source_models in [false, true] {
            sandbox.source_models = source_models;
            for all_active in [false, true] {
                let mut histories = Vec::new();
                for side in [-1, 1] {
                    let (mut game, id) = lookout(&mut sandbox, &assets, all_active, side);
                    let mut history = Vec::new();
                    for _ in 0..640 {
                        game.step(0).unwrap();
                        let actor = game.roster.get(id);
                        assert_eq!(actor.attribute & ATTR_PHASE, 0);
                        assert_eq!(game.intelligence.alert, 0);
                        assert_eq!(actor.position, Position::ZERO);
                        history.push((actor.rotation[1], actor.brain.clone()));
                    }
                    println!(
                        "kind {kind:#x}, T2 body {source_models}, all active {all_active}, side {side}: yaw {}",
                        game.roster.get(id).rotation[1]
                    );
                    assert!(history.iter().any(|(yaw, _)| *yaw != 0));
                    histories.push(history);
                    let forward = game.math.velocity(game.roster.get(id).rotation[1], 6000, 0);
                    game.teleport_player(
                        Position::new(i32::from(forward[0]), 0, i32::from(forward[2])),
                        true,
                    )
                    .unwrap();
                    for _ in 0..64 {
                        game.step(0).unwrap();
                        if game.roster.get(id).attribute & ATTR_PHASE == 2 {
                            break;
                        }
                    }
                    assert_eq!(
                        game.roster.get(id).attribute & ATTR_PHASE,
                        2,
                        "lookout must still detect a nearby visible player"
                    );
                }
                let different = histories[0]
                    .iter()
                    .zip(&histories[1])
                    .filter(|(a, b)| a != b)
                    .count();
                assert_eq!(
                    different, 0,
                    "kind {kind:#x}, T2 body {source_models}, all active {all_active}: unseen player changed lookout behavior"
                );
            }
        }
    }
}

fn stationary_arena() -> Sandbox {
    let mut sandbox = sandbox();
    let layout = sandbox.layout.as_mut().unwrap();
    let stationary = layout
        .actors
        .iter()
        .find(|a| a.route.is_empty())
        .unwrap()
        .clone();
    println!(
        "sequence {}, stationary slot {}, source behavior {:#x}",
        layout.sequence, stationary.slot, stationary.behavior
    );
    layout.actors = vec![stationary];
    layout.actors[0].position = [0; 3];
    layout.actors[0].facing = 0;
    sandbox.level.scene.statics.clear();
    sandbox.level.collision = tenchu_assets::tenchu2::collision::Collision {
        gaisen: None,
        origin: [-6000; 2],
        dimensions: [1; 2],
        cell_size: 12000,
        cells: vec![tenchu_assets::tenchu2::collision::Cell {
            minimum: i16::MIN,
            maximum: i16::MAX,
            starts: [0; 4],
            nodes: vec![tenchu_assets::terrain::Node {
                y: 0,
                dy: i16::MAX,
                bounds: [-6000, -6000, 6000, 6000],
                attributes: 0,
                division: u16::MAX,
            }],
        }],
    };
    sandbox
}

#[test]
#[ignore = "requires both discs; real melee contacts retain controller damage, HUD health and unalerted kills"]
fn sandbox_melee_and_displayed_health_match_controller() {
    let assets = assets();
    let mut sandbox = stationary_arena();
    let layout = sandbox.layout.as_mut().unwrap();
    layout.start = [0; 3];
    layout.facing = 0;
    layout.actors[0].position = [0, 0, -600];
    for kind in [0x10, 0x11, 0x12] {
        sandbox.guard = Some(kind);
        for character in 0..2 {
            for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
                for armed in [false, true] {
                    let mut outcomes = Vec::new();
                    for source_models in [false, true] {
                        sandbox.source_models = source_models;
                        let mut game = sandbox
                            .load(
                                &assets,
                                Options {
                                    character,
                                    difficulty,
                                    ..Default::default()
                                },
                            )
                            .unwrap();
                        let id = *game
                            .roster
                            .order
                            .iter()
                            .find(|&&id| id != game.player_id)
                            .unwrap();
                        game.roster.keep_all_active = true;
                        game.intelligence.ignore_player = true;
                        game.roster.get_mut(id).setup_think(0).unwrap();
                        for _ in 0..30 {
                            game.step(0).unwrap();
                        }
                        if armed {
                            game.roster.get_mut(id).attribute |= ATTR_WEAPON_DRAWN;
                        }
                        let initial = game.roster.get(id).health;
                        for tick in 0..90 {
                            game.step(if tick == 0 { input::SQUARE } else { 0 })
                                .unwrap();
                            if game.roster.get(id).health < initial {
                                break;
                            }
                        }
                        let actor = game.roster.get(id);
                        assert!(
                            actor.health < initial,
                            "no hit: {kind:#x}, hero {character}, armed {armed}, T2 body {source_models}"
                        );
                        if armed {
                            assert!(
                                actor.health > 0,
                                "ordinary first attack killed an armed guard"
                            );
                            let bar = game
                                .hud
                                .bars
                                .iter()
                                .find(|b| b.time > 0 && b.actor == id)
                                .unwrap();
                            assert_eq!((bar.health, bar.maximum), (actor.health, actor.max_health));
                        } else {
                            assert_eq!(actor.health, 0, "unalerted source kill");
                        }
                        outcomes.push((initial, actor.health));
                    }
                    assert_eq!(
                        outcomes[0], outcomes[1],
                        "body changed HP/damage: {kind:#x}, hero {character}, {difficulty:?}, armed {armed}"
                    );
                    println!(
                        "kind {kind:#x}, hero {character}, {difficulty:?}, armed {armed}: {} -> {} HP",
                        outcomes[0].0, outcomes[0].1
                    );
                }
            }
        }
    }
}
