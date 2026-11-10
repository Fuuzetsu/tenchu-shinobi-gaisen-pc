use crate::math::Position;
use crate::{
    game::{
        Game,
        events::{Events, Outcome},
    },
    ids::*,
    oracle,
};
use tenchu_assets::script::{ActorCommand, Command};
mod camera;

#[test]
#[ignore = "requires TENCHU_DISC; compares directed scene ticks with the original full update, including entry cues and the audio tail"]
fn directed_cinematics_match_full_source_updates() {
    for (stage, sequence) in [(0, 0), (7, 10), (10, 5)] {
        for character in 0..2 {
            let mut original = oracle::game(stage, character);
            let mut directed = oracle::game(stage, character);
            for game in [&mut original, &mut directed] {
                let event = &mut game.mission.scripts.events[0];
                event.sequence = sequence;
                event.mode = 0;
                game.events =
                    Events::new(&game.mission.scripts.events, &game.roster, game.player_id);
            }
            for tick in 0..6000 {
                let input = if tick == 0 { crate::input::RIGHT } else { 0 };
                original.step(input).unwrap();
                assert!(directed.step_cinematic(input).unwrap());
                assert_eq!(original.music, directed.music);
                assert_eq!(original.effects.sounds, directed.effects.sounds);
                assert_eq!(original.intelligence.voices, directed.intelligence.voices);
                assert_eq!(original.random.seed, directed.random.seed);
                assert_eq!(original.clock, directed.clock);
                assert_eq!(original.rendered_view, directed.rendered_view);
                assert_eq!(original.cinematic.active, directed.cinematic.active);
                assert_eq!(original.cinematic.frame, directed.cinematic.frame);
                assert_eq!(original.cinematic.elapsed, directed.cinematic.elapsed);
                assert_eq!(
                    original.cinematic.displayed_caption,
                    directed.cinematic.displayed_caption
                );
                for &id in &original.roster.order {
                    let a = original.roster.get(id);
                    let b = directed.roster.get(id);
                    assert_eq!(
                        (a.position, a.velocity, a.rotation, a.health, a.status),
                        (b.position, b.velocity, b.rotation, b.health, b.status)
                    );
                    assert_eq!(
                        (a.motion.action, a.motion.count, a.motion.loops),
                        (b.motion.action, b.motion.count, b.motion.loops)
                    );
                }
                if !original.cinematic.active {
                    assert!(tick > 0);
                    assert_eq!(
                        serde_json::to_value(&original.roster).unwrap(),
                        serde_json::to_value(&directed.roster).unwrap()
                    );
                    break;
                }
                assert!(tick < 5999, "unfinished source scene");
            }
        }
    }
}

fn finish(game: &mut Game) -> u64 {
    let start = game.tick;
    while game.cinematic.active {
        assert!(
            game.tick - start < 5000,
            "sequence {} did not finish",
            game.cinematic.sequence
        );
        game.step(0).unwrap_or_else(|e| {
            panic!(
                "sequence {} tick {}: {e}",
                game.cinematic.sequence, game.tick
            )
        });
    }
    assert!(!game.intelligence.cinematic);
    for &id in &game.roster.order {
        assert!(!game.roster.get(id).scripted);
    }
    game.tick - start
}

#[test]
#[ignore = "requires TENCHU_DISC; runs Mission 1's actual encounter and ending scripts for both characters"]
fn mission_one_cinematics_run_with_live_actors() {
    for character in 0..2 {
        for sequence in [0, 150, 151, 152, 220, 250] {
            let mut game = oracle::game(0, character);
            assert!(
                game.begin_cinematic(sequence).unwrap(),
                "character {character}, sequence {sequence}"
            );
            let ticks = finish(&mut game);
            println!("character {character}, sequence {sequence}: {ticks} ticks");
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; enters the first boss zone through the authored event chain"]
fn mission_one_boss_activation_restores_the_mission_clock() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let boss = game.roster.find(0x81).unwrap();
        assert_eq!(game.roster.get(boss).health, -1);
        game.events = Events::new(&game.mission.scripts.events, &game.roster, game.player_id);
        game.step(0).unwrap();
        assert_eq!(game.music, [Some(100)]);
        assert!(!game.cinematic.active);
        let clock = game.clock;
        let player = game.roster.get_mut(game.player_id);
        player.position = Position::new(1000, 0, 5000);
        game.scene
            .update(player.root_model, player.root(&game.math));
        game.step(0).unwrap();
        assert!(game.cinematic.active);
        assert_eq!(game.cinematic.sequence, 0);
        let ticks = 1 + finish(&mut game);
        assert!(game.roster.get(boss).health > 0);
        assert_eq!(game.roster.get(boss).attribute & ATTR_SUSPEND, 0);
        assert_eq!(game.clock, clock + i32::from(game.cinematic.elapsed));
        assert!(ticks >= game.cinematic.elapsed as u64);
        game.step(0).unwrap();
        assert_eq!(game.clock, clock + 1);
        assert_eq!(game.music, [Some(111)]);
        assert_eq!(game.events.outcome, Outcome::Running);
        assert_eq!(game.intelligence.action_halt, crate::ai::Halt::Running);
        assert!(game.tick > game.clock as u64);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; protects script-owned motion and the intentional audio tail from ordinary updates"]
fn controlled_motion_and_audio_tail_have_distinct_update_rules() {
    let mut game = oracle::game(0, 0);
    let mut bytes = Vec::new();
    for row in [
        [0i16, 42, 0, 0, 0, 123],
        [3, 0, MOT_NORMAL, 0, 0, 0],
        [1, 200, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
    ] {
        bytes.extend(row.into_iter().flat_map(i16::to_le_bytes));
    }
    bytes.extend((-1i16).to_le_bytes());
    game.mission.scripts.cutscenes = tenchu_assets::script::Cutscenes::parse(bytes).unwrap();
    game.input = crate::input::LEFT;
    assert!(game.begin_cinematic(42).unwrap());
    // A poison reaction can clear ActionHalt during a script; PadProc still does not poll.
    game.intelligence.action_halt = crate::ai::Halt::Running;
    game.step(crate::input::RIGHT).unwrap();
    assert_eq!(game.player().pad.held, crate::input::LEFT);
    game.intelligence.action_halt = crate::ai::Halt::Active;
    for _ in 0..198 {
        game.step(crate::input::UP | crate::input::SQUARE).unwrap();
        assert_eq!(game.player().motion.action, MOT_NORMAL);
        assert!(game.player().scripted);
    }
    game.step(0).unwrap();
    assert_eq!(game.cinematic.tail, 30);
    assert!(!game.player().scripted);
    assert_eq!(game.player().motion.action, MOT_ENGAGE_STANCE);
    let frozen = (
        game.clock,
        game.player().position,
        game.random.calls,
        game.rendered_view,
    );
    for i in 0..30 {
        game.step(crate::input::UP | crate::input::SQUARE).unwrap();
        assert_eq!(
            frozen,
            (
                game.clock,
                game.player().position,
                game.random.calls,
                game.rendered_view
            )
        );
        assert_eq!(game.cinematic.active, i != 29);
    }
    assert_eq!(game.music, [None]);
    assert_eq!(game.tick, 230);
    game.step(0).unwrap();
    assert_eq!(game.clock, 201);
}

fn mark_dead(game: &mut Game, kind: i16) {
    assert!(
        game.cinematic_actor(
            kind,
            ActorCommand::Animate {
                motion: MOT_DEAD,
                loops: 0,
                next: -1
            }
        )
        .unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; follows the real Mission 1 boss-death routes to completion and game over"]
fn mission_one_routes_finish_after_boss_deaths() {
    for character in 0..2 {
        for player_dies in [false, true] {
            let mut game = oracle::game(0, character);
            game.events = Events::new(&game.mission.scripts.events, &game.roster, game.player_id);
            game.roster.get_mut(game.player_id).position = Position::new(1000, 0, 5000);
            game.step(0).unwrap();
            finish(&mut game);
            game.step(0).unwrap();
            if player_dies {
                mark_dead(&mut game, character);
                for _ in 0..1000 {
                    game.step(0).unwrap();
                    if game.events.outcome != Outcome::Running {
                        break;
                    }
                }
                assert_eq!(game.events.outcome, Outcome::GameOver);
                continue;
            }
            mark_dead(&mut game, 0x81);
            game.step(0).unwrap();
            assert_eq!(game.cinematic.sequence, 220);
            finish(&mut game);
            game.roster.get_mut(game.player_id).position = Position::new(-33000, -1000, 5000);
            game.step(0).unwrap();
            assert_eq!(game.cinematic.sequence, 150);
            finish(&mut game);
            game.step(0).unwrap();
            let merchant = game.roster.find(0x80).unwrap();
            assert!(game.roster.get(merchant).health > 0);
            mark_dead(&mut game, 0x80);
            game.step(0).unwrap();
            assert_eq!(game.cinematic.sequence, 151);
            finish(&mut game);
            game.step(0).unwrap();
            assert_eq!(game.events.outcome, Outcome::Complete);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; runs every English sequence in isolation as a coverage diagnostic"]
fn campaign_cinematic_coverage() {
    let mut failures = Vec::new();
    let mut sequences = 0;
    for stage in 0..11 {
        for character in 0..2 {
            let game = oracle::game(stage, character);
            let ids: Vec<_> = game
                .mission
                .scripts
                .cutscenes
                .commands
                .iter()
                .filter_map(|c| {
                    if let Command::Sequence { id, .. } = *c {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();
            for sequence in ids {
                let mut game = oracle::game(stage, character);
                let result = (|| -> crate::Result<()> {
                    if !game.begin_cinematic(sequence)? {
                        return Ok(());
                    }
                    while game.cinematic.active {
                        if game.tick > 33000 {
                            return Err("sequence exceeded signed wait range".into());
                        }
                        game.step(0)?;
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    failures.push(format!("stage {stage}, character {character}, sequence {sequence}, tick {}: {error}",game.tick));
                }
                sequences += 1;
            }
        }
    }
    for failure in &failures {
        eprintln!("{failure}");
    }
    assert!(
        failures.is_empty(),
        "{} failures across {sequences} sequences",
        failures.len()
    );
    println!("Executed {sequences} English sequence entries");
}

#[test]
#[ignore = "requires TENCHU_DISC; map input leaves the world running, uses the local sound, closes cleanly and persists its animation state through cinematics"]
fn map_input_lifetime_and_cinematic_suppression() {
    use crate::{
        input::{SELECT, UP},
        oracle,
    };
    let mut game = oracle::game(0, 0);
    for _ in 0..60 {
        game.step(0).unwrap();
    }
    game.step(SELECT).unwrap();
    assert_eq!((game.hud.map.mode, game.hud.map.slide), (1, 160));
    assert!(
        game.sounds()
            .unwrap()
            .iter()
            .any(|s| s.id == 0x27 && s.volume == 126 && s.pan == 64)
    );
    let before = game.player().position;
    for _ in 0..30 {
        game.step(SELECT | UP).unwrap();
    }
    assert_ne!(game.player().position, before);
    assert_eq!(game.hud.map.mode, 2);
    assert_eq!(game.hud.map.lines.len(), 2);
    game.begin_cinematic(0).unwrap();
    game.step(SELECT).unwrap();
    assert!(game.cinematic.active);
    assert_eq!(game.hud.map.dim, None);
    assert!(game.hud.map.lines.is_empty());
    assert_eq!(game.hud.map.mode, 2);
    for _ in 0..2000 {
        game.step(0).unwrap();
        if !game.cinematic.active {
            break;
        }
    }
    assert!(!game.cinematic.active);
    game.step(0).unwrap();
    assert_eq!(game.hud.map.mode, 0);
    assert_eq!(game.hud.map.dim, None);
    game.step(SELECT).unwrap();
    assert_eq!((game.hud.map.mode, game.hud.map.slide), (1, 160));
}
