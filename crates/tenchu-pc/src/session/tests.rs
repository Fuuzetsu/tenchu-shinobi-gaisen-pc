use super::*;
use std::time::Duration;
use tenchu_core::{clock::GameClock, input::*};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_ROSTER_TRACE; compares persistent roster fields with retail RAM"]
fn roster_and_weapon_definitions_match_retail_capture() {
    use std::io::Read;
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut file = std::fs::File::open(
        std::env::var_os("TENCHU_ROSTER_TRACE").expect("Set TENCHU_ROSTER_TRACE"),
    )
    .unwrap();
    let mut prefix = vec![0; 96 + 0x200000];
    file.read_exact(&mut prefix).unwrap();
    assert!(matches!(&prefix[..8], b"ONKTR001" | b"ONKTR002"));
    assert_eq!(&prefix[16..24], b"ONKRAM01");
    let ram = &prefix[96..];
    let bytes = |address: u32, n: usize| &ram[(address & 0x1fffff) as usize..][..n];
    let short = |address| i16::from_le_bytes(bytes(address, 2).try_into().unwrap());
    let word = |address| u32::from_le_bytes(bytes(address, 4).try_into().unwrap());
    let stage = word(0x80097ac4) as usize;
    let player = word(0x80097c60);
    let mission = assets
        .stages()
        .unwrap()
        .into_iter()
        .find(|s| s.id == stage)
        .unwrap()
        .order as usize;
    let game = Options {
        character: short(player),
        mission,
        layout: usize::from(bytes(0x80010006, 1)[0]) + 1,
        ..Options::default()
    }
    .load(&assets)
    .unwrap();
    assert_eq!(game.roster.order.len(), short(0x80097724) as usize);
    assert_eq!(
        [
            game.statistics.bosses,
            game.statistics.enemies,
            game.statistics.citizens
        ],
        [short(0x80097c74), short(0x80097c76), short(0x80097c78)]
    );
    for (i, &id) in game.roster.order.iter().enumerate() {
        let ptr = word(0x800be7b8 + i as u32 * 4);
        let actor = game.roster.get(id);
        assert_eq!(actor.kind, short(ptr), "roster position {i}");
        assert_eq!(
            actor.max_health,
            short(ptr + 10),
            "actor {i} maximum health"
        );
        // CVA moves the player/story actors and rewrites their home after construction.
        if id != game.player_id && !game.mission.story.iter().any(|s| s.kind == actor.kind) {
            assert_eq!(
                actor.home,
                [word(ptr + 0x78) as i32, word(ptr + 0x7c) as i32],
                "actor {i} layout home"
            );
        }
        let trace = word(ptr + 0x70);
        assert_eq!(
            actor.patrol.is_some(),
            trace != 0,
            "actor {i} patrol presence"
        );
        if let Some(patrol) = &actor.patrol {
            let points = word(trace + 4);
            for (j, point) in patrol.points.iter().enumerate() {
                let p = points + j as u32 * 12;
                assert_eq!(
                    (point.position, point.range, point.pad),
                    (
                        [word(p) as i32, word(p + 4) as i32],
                        short(p + 8),
                        short(p + 10) as u16
                    ),
                    "actor {i}, patrol point {j}"
                );
            }
        }
        for hand in 0..2 {
            assert_eq!(
                actor.weapon_hitbox[hand].map_or(-1, |x| x as i16),
                short(ptr + 0x90 + hand as u32 * 2),
                "actor {i} hand {hand}"
            );
        }
    }
}

fn state(game: &Game) -> String {
    format!(
        "{:?}",
        (
            &game.roster,
            &game.random,
            &game.effects,
            &game.scene,
            &game.collision,
            &game.camera,
            &game.intelligence,
            &game.visible,
            &game.activation,
            &game.drawing,
            &game.combat,
            &game.statistics,
        )
    )
}

#[test]
#[ignore = "requires TENCHU_DISC pointing to the private SLPS-019.01 disc"]
fn retail_movement_is_independent_of_host_frame_cadence() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let tape: Vec<_> = [
        (30, 0),
        (30, UP),
        (1, UP | CROSS),
        (30, UP),
        (30, 0),
        (20, RIGHT),
        (20, UP),
        (30, R1),
        (30, UP | R1),
        (60, 0),
        (15, L1),
        (20, L1 | RIGHT),
        (20, L1 | UP),
        (15, L1),
        (30, 0),
        (1, SQUARE),
        (9, 0),
        (1, SQUARE),
        (9, 0),
        (1, SQUARE),
        (9, 0),
        (1, SQUARE),
        (60, 0),
        (1, RIGHT | SQUARE),
        (60, 0),
        (1, LEFT | SQUARE),
        (60, 0),
    ]
    .into_iter()
    .flat_map(|(count, input)| std::iter::repeat_n(input, count))
    .collect();
    for character in 0..=1 {
        let options = Options {
            character,
            layout: 1,
            ..Options::default()
        };
        let mut reference = options.load(&assets).unwrap();
        let mut states = Vec::new();
        let mut motions = std::collections::BTreeSet::new();
        let mut looked = false;
        for &input in &tape {
            reference.step(input).unwrap();
            motions.insert(reference.player().motion.action);
            looked |= reference.camera.mode == tenchu_core::camera::Mode::Direction;
            states.push(state(&reference));
        }
        assert!(
            motions.iter().any(|motion| motion >> 8 == 9),
            "jump was not reached"
        );
        assert!(
            motions.iter().any(|motion| motion >> 8 == 11),
            "crouch was not reached"
        );
        assert!(
            motions.contains(&tenchu_core::ids::MOT_ATTACK),
            "melee attack was not reached"
        );
        assert!(looked, "direction camera was not reached");
        assert_eq!(reference.camera.mode, tenchu_core::camera::Mode::Normal);
        for fps in [24, 144] {
            let mut game = options.load(&assets).unwrap();
            let mut clock = GameClock::default();
            for frame in 0..(tape.len() as u64 * fps).div_ceil(30) + 1 {
                let elapsed = (frame + 1) * 1_000_000_000 / fps - frame * 1_000_000_000 / fps;
                clock.elapse(Duration::from_nanos(elapsed));
                while let Some(tick) = clock.step() {
                    if tick as usize == tape.len() {
                        break;
                    }
                    let tick = tick as usize;
                    game.step(tape[tick]).unwrap();
                    assert_eq!(
                        state(&game),
                        states[tick],
                        "character {character}, {fps} Hz, tick {tick}"
                    );
                }
                if game.tick as usize == tape.len() {
                    break;
                }
            }
            assert_eq!(game.tick as usize, tape.len());
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; constructs every retail mission, character and layout"]
fn all_retail_layouts_construct_actor_and_collision_pools() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for mission in 0..=10 {
        for character in 0..=1 {
            for layout in 1..=3 {
                let game = Options {
                    mission,
                    character,
                    layout,
                    ..Options::default()
                }
                .load(&assets)
                .unwrap_or_else(|error| {
                    panic!("mission {mission}, character {character}, layout {layout}: {error}")
                });
                assert_eq!(game.mission.stage.order as usize, mission);
                assert_eq!(game.roster.order[0], game.player_id);
                assert_eq!(game.collision.len, game.roster.order.len());
                assert_eq!(
                    game.roster.actors.iter().flatten().count(),
                    game.roster.order.len()
                );
                for &id in &game.roster.order {
                    let actor = game.roster.get(id);
                    let waist = actor.parts[0];
                    let slot = game.scene.models[waist].conflict.unwrap();
                    assert_eq!(game.collision.slots[slot].owner, Some(id));
                    if let Some(patrol) = &actor.patrol {
                        assert!(patrol.points.len() > 1);
                        assert_eq!(patrol.points.last().unwrap().pad, 0xffff);
                    }
                }
                if mission == 1 && layout == 1 {
                    println!(
                        "Mission 1 character {character}: {} actors, {:?}",
                        game.roster.order.len(),
                        game.statistics
                    );
                }
            }
        }
    }
}
