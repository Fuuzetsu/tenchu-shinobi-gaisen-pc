use super::*;
use tenchu_assets::tenchu2::campaign::Objective;
use tenchu_core::tenchu2::input::JUMP;

mod death;

fn start(sequence: usize) -> Playback<crate::source::DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut loadout = [0; 21];
    loadout[0] = 255;
    loadout[4] = 5;
    Playback::load(disc, sequence, 2, 17, Some(loadout), None).unwrap()
}

fn reach_exit(playback: &mut Playback<crate::source::DiscSource>) {
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not gameplay")
    };
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
}

fn next(playback: &mut Playback<crate::source::DiscSource>) {
    let generation = playback.generation;
    for _ in 0..6000 {
        playback.step(0).unwrap();
        if playback.generation != generation {
            return;
        }
    }
    panic!("sequence {} did not finish", playback.run.sequence);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; shared activation and stance preferences survive native area reloads and restore their defaults"]
fn gameplay_preferences_survive_native_area_reload() {
    let mut playback = start(9);
    let mut settings = crate::settings::Settings {
        keep_enemies_active: true,
        circle_action: tenchu_core::input::CircleAction::Crouch,
        ..Default::default()
    };
    playback.configure(&settings);
    for _ in 0..2 {
        playback.step(0).unwrap();
        let Content::Gameplay(session) = &playback.content else {
            unreachable!()
        };
        assert!(session.world.keep_all_active);
        assert_eq!(session.circle_action, settings.circle_action);
        assert!(session.world.actors.iter().all(|a| a.actor.flags & 8 == 0));
        playback.load_current().unwrap();
    }
    settings.keep_enemies_active = false;
    settings.circle_action = Default::default();
    playback.configure(&settings);
    playback.step(0).unwrap();
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    assert!(!session.world.keep_all_active);
    assert_eq!(session.circle_action, settings.circle_action);
    assert!(session.world.actors.iter().any(|a| a.actor.flags & 8 != 0));
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; disc mission menu, all three campaigns, music, saved supplies and equipment cancellation"]
fn native_mission_menu_enters_each_campaign_from_its_opening_scene() {
    use super::super::selection::Edit;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut playback = Playback::menu(disc, 3, 17, None).unwrap();
    for hero in 0..3 {
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        for _ in 0..hero {
            playback.action = Some(Action::MissionSelection(Edit::Hero(true)));
            playback.step(0).unwrap();
            for _ in 0..11 {
                playback.step(0).unwrap();
            }
        }
        let Content::MissionSelection(selection) = &mut playback.content else {
            panic!("not mission menu")
        };
        assert_eq!(selection.state.hero, hero);
        assert_eq!(selection.state.displayed, hero);
        assert!(!selection.music.frames.is_empty());
        assert!(selection.music.frames.iter().flatten().any(|&s| s != 0));
        assert!(selection.motions[hero].frame >= 0);
        assert_eq!(selection.resources.names.size, [208, 240]);
        for mission in 0..11 {
            assert_eq!(
                selection
                    .state
                    .available(mission, &selection.resources.rules),
                playback.campaign.starts[hero][mission].is_some()
            );
        }
        selection.state.progress.stock[hero].items[4] = Some(23);
        playback.action = Some(Action::MissionSelection(Edit::Mission(1)));
        playback.step(0).unwrap();
        playback.action = Some(Action::Continue);
        playback.step(0).unwrap();
        next(&mut playback);
        assert_eq!(playback.run.hero, hero);
        assert_eq!(playback.run.mission, 1);
        assert_eq!(playback.run.difficulty, 3);
        assert_eq!(
            playback.run.sequence,
            playback.campaign.starts[hero][1].unwrap()
        );
        if hero != 2 {
            let Content::Prologue(intro) = &playback.content else {
                panic!("missing training prologue")
            };
            assert!(
                intro
                    .audio
                    .frames
                    .iter()
                    .flatten()
                    .any(|&sample| sample != 0)
            );
            if hero == 1 {
                for _ in 0..40 {
                    playback.step(0).unwrap();
                }
                playback.action = Some(Action::Continue);
                playback.step(0).unwrap();
            }
            next(&mut playback);
        }
        assert!(matches!(playback.content, Content::Story { .. }));
        for tick in 0..1500 {
            if matches!(playback.content, Content::Equipment(_)) {
                break;
            }
            assert!(
                !playback
                    .step(if tick % 60 == 0 { 0x4000 } else { 0 })
                    .unwrap()
                    .quit
            );
        }
        let Content::Equipment(selection) = &playback.content else {
            panic!("opening did not reach equipment")
        };
        assert_eq!(selection.state.stock.items[4], Some(23));
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        playback.action = Some(Action::Quit);
        playback.step(0).unwrap();
        next(&mut playback);
        assert!(matches!(playback.content, Content::MissionSelection(_)));
        assert_eq!(playback.run.progress.stock[hero].items[4], Some(23));
    }
    let Content::MissionSelection(selection) = &mut playback.content else {
        unreachable!()
    };
    selection.state.progress.records[0][0].time_tenths = 1200;
    for _ in 0..15 {
        playback.step(0).unwrap();
    }
    playback.action = Some(Action::Continue);
    playback.step(0).unwrap();
    next(&mut playback);
    assert!(
        matches!(playback.content, Content::Story { .. }),
        "Completed training should skip the prologue"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; original equipment, stock, menu audio, training bypass and boss-area carry"]
fn native_equipment_supplies_the_campaign_inventory() {
    use super::super::equipment::Edit;
    let disc =
        || crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, counts) in [(1, [(1, 0), (4, 0)]), (4, [(1, 1), (4, 5)])] {
        let playback = Playback::load(disc(), sequence, 2, 17, None, None).unwrap();
        let Content::Gameplay(session) = &playback.content else {
            panic!("training equipment did not bypass the menu")
        };
        for (item, count) in counts {
            assert_eq!(session.world.actors[0].actor.inventory.counts[item], count);
        }
    }
    let mut playback = Playback::load(disc(), 9, 2, 17, None, None).unwrap();
    assert!(matches!(playback.content, Content::Equipment(_)));
    for _ in 0..15 {
        playback.step(0).unwrap();
    }
    let Content::Equipment(selection) = &playback.content else {
        unreachable!()
    };
    assert_eq!(selection.state.fade, 0);
    let mut mixer = tenchu_audio::Mixer::new(44100);
    mixer.set_bank(selection.bank.clone());
    let sounds = playback.step(0x2000).unwrap().sounds;
    assert_eq!(sounds.len(), 1);
    for event in sounds {
        let sound = event.sound;
        mixer.play_at(0, i16::from(sound.id), sound.volume, sound.note);
    }
    assert!(
        (0..4410)
            .flat_map(|_| mixer.frame())
            .any(|s| s.abs() > 0.01)
    );
    playback.step(0).unwrap();
    playback.action = Some(Action::Equipment(Edit::Select(3)));
    playback.step(0).unwrap();
    for _ in 0..3 {
        playback.action = Some(Action::Equipment(Edit::Add));
        playback.step(0).unwrap();
    }
    playback.action = Some(Action::Equipment(Edit::Remove));
    playback.step(0).unwrap();
    let Content::Equipment(selection) = &playback.content else {
        unreachable!()
    };
    assert_eq!(selection.state.remaining, 9);
    assert_eq!(selection.state.stock.items[1], Some(2));
    assert_eq!(selection.state.stock.items[4], Some(8));
    playback.action = Some(Action::Continue);
    playback.step(0).unwrap();
    next(&mut playback);
    let Content::Gameplay(session) = &playback.content else {
        panic!("equipment did not enter gameplay")
    };
    assert_eq!(session.world.actors[0].actor.inventory.counts[1], 1);
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 2);
    assert_eq!(playback.run.progress.stock[0].items[4], Some(8));
    reach_exit(&mut playback);
    next(&mut playback);
    playback.step(0x2000).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 11);
    let Content::Gameplay(session) = &playback.content else {
        panic!("boss entry reopened equipment")
    };
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 2);

    let mut playback = Playback::load(disc(), 9, 2, 17, None, None).unwrap();
    for _ in 0..15 {
        playback.step(0).unwrap();
    }
    playback.step(0x2000).unwrap();
    playback.action = Some(Action::Quit);
    playback.step(0).unwrap();
    let mut quit = false;
    for _ in 0..20 {
        quit |= playback.step(0).unwrap().quit;
    }
    assert!(quit);
    assert_eq!(playback.run.progress.stock[0].items[1], Some(3));
    assert_eq!(playback.run.carry.inventory[0], 0);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; armor selects each source costume and activates the damage rule without replacing the weapon"]
fn native_equipment_armor_uses_the_original_costumes() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    for (hero, map, name) in [
        (0, 2, "rikia"),
        (1, 3, "ayamea"),
        (2, 12, "tatua_m"),
        (2, 14, "tatua"),
    ] {
        let layout = layouts
            .iter()
            .find(|l| l.map == map && campaign.mission(hero, l.sequence).is_some())
            .unwrap();
        let mut run = Run::new(&campaign, hero, layout.sequence, 2, 17).unwrap();
        run.prepare_area(&campaign).unwrap();
        run.carry.inventory[0] = 1;
        run.carry.alternate = true;
        let mut session = Session::for_run(&disc, &run).unwrap();
        let definition = session
            .assets
            .resources
            .registry
            .actor(hero as i16)
            .unwrap();
        assert_eq!(definition.model, name);
        assert_eq!(definition.skill, 5);
        assert_eq!(
            definition.weapon,
            tenchu_assets::tenchu2::registry::Registry::load(&disc)
                .unwrap()
                .actor(hero as i16)
                .unwrap()
                .weapon
        );
        assert!(session.assets.models.contains_key(name));
        assert!(session.world.controls.armor);
        assert_eq!(session.world.actors[0].actor.inventory.counts[19], 0);
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        assert!(session.pending.is_empty());
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; results persist once and reopen with the same records, supplies and unlocked inventory"]
fn native_results_survive_restarting_the_campaign() {
    use super::super::save::SaveFile;
    let path = std::env::temp_dir().join(format!(
        "tenchu-jp-results-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut playback = start(134);
    playback.save = Some(SaveFile::at(path.clone()));
    playback.run.progress.stock[1].items[7] = Some(12);
    playback.run.progress.tatsumaru_unlocked = true;
    reach_exit(&mut playback);
    next(&mut playback);
    assert!(matches!(playback.content, Content::Results(_)));
    let expected = serde_json::to_value(&playback.run.progress).unwrap();
    let saved = std::fs::read(&path).unwrap();
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    drop(playback);

    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut loadout = [0; 21];
    loadout[0] = 255;
    let reopened = Playback::load(
        disc,
        134,
        2,
        17,
        Some(loadout),
        Some(SaveFile::at(path.clone())),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&reopened.run.progress).unwrap(),
        expected
    );
    let Content::Gameplay(session) = &reopened.content else {
        panic!("not gameplay")
    };
    assert!(session.world.controls.unlocked_items[7]);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires TENCHU2_DISC; live exits, results, next mission, cutscene skipping and new-area reset"]
fn native_playback_advances_through_results_into_the_next_mission() {
    let mut playback = start(134);
    reach_exit(&mut playback);
    next(&mut playback);
    assert!(matches!(playback.content, Content::Results(_)));
    assert_eq!(playback.run.sequence, 135);
    assert_eq!(playback.run.results.as_ref().unwrap().record.score, 400);
    let stock = playback.run.progress.stock.clone();
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    assert_eq!(
        serde_json::to_value(stock).unwrap(),
        serde_json::to_value(&playback.run.progress.stock).unwrap()
    );
    playback.action = Some(Action::Continue);
    playback.step(0).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 136);
    assert!(matches!(playback.content, Content::Story { .. }));
    playback.step(JUMP).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 137);
    let Content::Gameplay(session) = &playback.content else {
        panic!("not gameplay")
    };
    assert_eq!(session.world.actors[0].actor.kind, 1);
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 5);
    assert_eq!(session.world.statistics.killed, 0);
    assert_eq!(playback.run.summary.time_tenths, 0);
}

#[test]
#[ignore = "requires TENCHU2_DISC; live mission/story/boss/results and failure-story/retry transitions"]
fn native_playback_follows_boss_success_and_failure_branches() {
    let mut playback = start(9);
    if let Content::Gameplay(session) = &mut playback.content {
        session.world.actors[0].actor.health = 61;
        session.world.actors[0].actor.inventory.counts[4] = 3;
        session.world.statistics.stealth_kills = 2;
        session.world.statistics.killed = 2;
    }
    reach_exit(&mut playback);
    next(&mut playback);
    assert_eq!(playback.run.sequence, 10);
    assert!(matches!(playback.content, Content::Story { .. }));
    playback.step(0x2000).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 11);
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not boss")
    };
    assert_eq!(session.world.actors[0].actor.health, 61);
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 3);
    assert_eq!(session.world.statistics.stealth_kills, 2);
    for index in session.world.mission.targets.into_iter().flatten() {
        session.world.actors[index].actor.health = 0;
    }
    next(&mut playback);
    assert_eq!(playback.run.sequence, 12);
    next(&mut playback);
    assert_eq!(playback.run.sequence, 13);
    assert!(matches!(playback.content, Content::Results(_)));
    assert_eq!(
        playback
            .run
            .results
            .as_ref()
            .unwrap()
            .summary
            .kills
            .stealth_kills,
        2
    );
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 9);
    assert_eq!(playback.run.summary.kills.killed, 0);

    let mut playback = start(11);
    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    session.world.actors[0].actor.health = 0;
    session.world.actors[0].actor.motion.loops = 1;
    next(&mut playback);
    assert_eq!(playback.run.sequence, 14);
    next(&mut playback);
    assert!(matches!(playback.content, Content::Failed));
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    assert_eq!(playback.run.sequence, 9);
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    assert_eq!(
        session.world.actors[0].actor.health,
        session.world.actors[0].actor.max_health
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; chapter narration and streamed movie completion/skip dispatch"]
fn native_playback_advances_through_introductions_and_movies() {
    use tenchu_core::tenchu2::introduction::Phase;
    let mut playback = start(13);
    playback.action = Some(Action::Continue);
    playback.step(0).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 15);
    let Content::Introduction(intro) = &playback.content else {
        panic!("not introduction")
    };
    assert_eq!(intro.state.phase, Phase::Title);
    let generation = playback.generation;
    let mut narration = 0;
    for _ in 0..6000 {
        let update = playback.step(0).unwrap();
        narration += usize::from(update.music.is_some());
        if playback.generation != generation {
            break;
        }
    }
    assert_eq!(narration, 1);
    assert!(matches!(playback.content, Content::Story { .. }));
    assert_eq!(playback.run.sequence, 16);

    let mut playback = start(34);
    let Content::Movie(movie) = &playback.content else {
        panic!("not movie")
    };
    let first = movie.frame.as_ref().unwrap().number;
    playback.step(0).unwrap();
    let Content::Movie(movie) = &playback.content else {
        panic!("not movie")
    };
    assert_eq!(movie.frame.as_ref().unwrap().number, first);
    playback.step(0).unwrap();
    let Content::Movie(movie) = &playback.content else {
        panic!("not movie")
    };
    assert_eq!(movie.frame.as_ref().unwrap().number, first + 1);
    next(&mut playback);
    assert_eq!(playback.run.sequence, 35);
    assert!(matches!(playback.content, Content::Results(_)));

    let mut playback = start(34);
    playback.step(0x2000).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 35);
}

#[test]
#[ignore = "requires TENCHU2_DISC; retail result buttons wait for release, play the menu cue and leave after sixty vblanks"]
fn native_results_buttons_follow_the_printed_legend() {
    for buttons in [0x2000, 8, 0x4000, 9] {
        let mut playback = start(13);
        playback.previous_buttons = buttons;
        for _ in 0..3 {
            assert!(playback.step(buttons).unwrap().sounds.is_empty());
        }
        playback.step(0).unwrap();
        let update = playback.step(buttons).unwrap();
        assert_eq!(
            update.sounds.iter().map(|s| s.sound).collect::<Vec<_>>(),
            [Sound {
                id: 2,
                slot: 0,
                note: 36,
                volume: [127; 2]
            }]
        );
        let Content::Results(result) = &playback.content else {
            unreachable!()
        };
        assert!(result.bank.programs[0].as_ref().unwrap().tones[2].sample > 0);
        for _ in 0..29 {
            let update = playback.step(buttons).unwrap();
            assert!(!update.quit && update.sounds.is_empty());
            assert_eq!(playback.run.sequence, 13);
        }
        let update = playback.step(buttons).unwrap();
        match buttons {
            0x2000 => assert_eq!(playback.run.sequence, 15),
            8 => assert_eq!(playback.run.sequence, 9),
            _ => assert!(update.quit),
        }
    }
}
