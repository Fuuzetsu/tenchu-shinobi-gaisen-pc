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
    let wall_clips: Vec<_> = game
        .mission
        .characters
        .iter()
        .flat_map(|character| character.motions.iter())
        .filter(|motion| motion.action == tenchu_core::ids::MOT_STICKON)
        .filter_map(|motion| {
            game.mission
                .clips
                .iter()
                .find(|clip| clip.id == motion.clip)
        })
        .collect();
    format!(
        "{:?}",
        (
            &game.roster,
            &game.random,
            &game.effects,
            &game.items,
            &game.scene,
            &game.collision,
            &game.camera,
            &game.intelligence,
            &game.visible,
            &game.activation,
            &game.drawing,
            (
                &game.combat,
                &game.statistics,
                &game.hud,
                wall_clips,
                game.clock,
                game.input,
                game.rendered_view,
                &game.cinematic,
                &game.events,
                &game.music,
            ),
        )
    )
}

#[test]
#[ignore = "requires TENCHU_DISC; validates that editable wall-pose tracks do not alias other original motion tracks"]
fn wall_pose_keys_have_distinct_storage_in_retail_assets() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let definitions = tenchu_assets::executable::Executable::new(&assets.executable)
        .unwrap()
        .characters()
        .unwrap();
    for (kind, file) in [(0, "RIKIMARU"), (1, "AYAME")] {
        let clip = definitions
            .iter()
            .find(|c| c.kind == kind)
            .unwrap()
            .motions
            .iter()
            .find(|m| m.action == tenchu_core::ids::MOT_STICKON)
            .unwrap()
            .clip;
        let data = assets.asset(&format!("HUMAN/MOTION/{file}.AMD")).unwrap();
        let word =
            |offset| u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let mut tracks = Vec::new();
        for i in 0..word(0) {
            let at = word(4 + i * 4);
            let id = i16::from_le_bytes(data[at + 6..at + 8].try_into().unwrap());
            for track in 0..=usize::from(data[at]) {
                tracks.push((id, track, at + word(at + 8 + track * 4)));
            }
        }
        for track in [0, 1, 3] {
            let offset = tracks
                .iter()
                .find(|t| t.0 == clip && t.1 == track)
                .unwrap()
                .2;
            let aliases: Vec<_> = tracks.iter().filter(|t| t.2 == offset).collect();
            assert_eq!(aliases.len(), 1, "{file}: {aliases:?}");
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; validates every campaign/training ESD and CAD for both characters in all four languages"]
fn all_stage_scripts_parse_from_the_disc() {
    use tenchu_assets::script::{Language, Scripts};
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut sequences = 0;
    let mut commands = 0;
    for stage in 0..11 {
        for character in 0..2 {
            for language in [
                Language::English,
                Language::French,
                Language::Italian,
                Language::Japanese,
            ] {
                let scripts =
                    Scripts::load(&assets, stage, character, language).unwrap_or_else(|e| {
                        panic!("stage {stage} character {character} {language:?}: {e}")
                    });
                assert!(scripts.events.iter().any(|e| e.id == 0));
                for event in &scripts.events {
                    for id in event.next {
                        if id != 255 {
                            assert!(scripts.events.iter().any(|e| e.id == id));
                        }
                    }
                }
                for command in &scripts.cutscenes.commands {
                    if command.operation == 0 {
                        sequences += 1;
                    }
                    if command.operation == 8 && command.id >= 0 {
                        assert!(
                            !language
                                .decode(scripts.cutscenes.text(command.id).unwrap())
                                .contains('\u{fffd}')
                        );
                    }
                }
                commands += scripts.cutscenes.commands.len();
            }
        }
    }
    println!("Decoded {sequences} sequences, {commands} commands across 88 CAD files");
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

#[test]
#[ignore = "requires TENCHU_DISC; compares scripted waits, audio-tail holds and return to combat at different display cadences"]
fn cinematic_timing_is_independent_of_host_frame_cadence() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        let mut snapshots = Vec::new();
        for cadence in [
            vec![16_666_667],
            vec![6_944_444],
            vec![3_000_000, 81_000_000, 900_000_000, 11_000_000],
        ] {
            let mut game = Options {
                character,
                ..Options::default()
            }
            .load(&assets)
            .unwrap();
            let player = game.roster.get_mut(game.player_id);
            player.position = [1000, 0, 5000];
            game.scene
                .update(player.root_model, player.root(&game.math));
            let mut clock = tenchu_core::clock::GameClock::default();
            let mut frame = 0;
            while game.tick < 1050 {
                clock.elapse(std::time::Duration::from_nanos(
                    cadence[frame % cadence.len()],
                ));
                while clock.step().is_some() {
                    let input = if game.tick % 12 == 0 { SQUARE } else { 0 };
                    game.step(input).unwrap();
                    if game.tick == 1050 {
                        break;
                    }
                }
                frame += 1;
            }
            assert!(game.clock < 200);
            assert!(!game.cinematic.active);
            snapshots.push(state(&game));
        }
        assert_eq!(snapshots[0], snapshots[1]);
        assert_eq!(snapshots[0], snapshots[2]);
    }
}
