use crate::{
    game::{
        Game,
        events::{Events, Outcome},
    },
    ids::*,
    oracle,
};
mod camera;

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
        player.position = [1000, 0, 5000];
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
        assert_eq!(game.intelligence.action_halt, 0);
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
    game.intelligence.action_halt = 0;
    game.step(crate::input::RIGHT).unwrap();
    assert_eq!(game.player().pad.held, crate::input::LEFT);
    game.intelligence.action_halt = 1;
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
        game.cinematic_actor(tenchu_assets::script::Command {
            operation: 3,
            id: kind,
            position: [MOT_DEAD, 0, -1],
            parameter: 0,
        })
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
            game.roster.get_mut(game.player_id).position = [1000, 0, 5000];
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
            game.roster.get_mut(game.player_id).position = [-33000, -1000, 5000];
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
                .filter(|c| c.operation == 0)
                .map(|c| c.id)
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
