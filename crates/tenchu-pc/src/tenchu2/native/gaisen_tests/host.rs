use super::campaign::{Content, Playback};
use super::*;
use tenchu_assets::script::Language;

#[test]
#[ignore = "requires both discs; runs original scripted encounters through native playback and audio for every source story/native hero"]
fn imported_host_runs_original_routes_and_audio() {
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    let mut sequences = 0;
    let mut sounds = 0;
    for stage in 0..11 {
        for character in 0..2 {
            for hero in 0..3 {
                let source =
                    gaisen::Source::open(path.as_ref(), stage, character, 1, Language::English)
                        .unwrap();
                let mut playback = Playback::gaisen(
                    crate::source::open_disc(t2.as_ref()).unwrap(),
                    source,
                    hero,
                    2,
                    1,
                    Some([0; 21]),
                    4000,
                )
                .unwrap();
                let mut scenes = BTreeSet::new();
                for _ in 0..20000 {
                    if matches!(playback.content, Content::Results(_)) {
                        break;
                    }
                    let Content::Gameplay(session) = &mut playback.content else {
                        panic!("Unexpected scene");
                    };
                    let scene = session.gaisen.as_ref().unwrap();
                    if scene.director.source.cinematic.active {
                        scenes.insert(scene.director.source.cinematic.sequence);
                    } else if !session.cinematic()
                        && scene.director.source.tick > 0
                        && session.world.mission.input_allowed()
                    {
                        director::force_next(&scene.director, &mut session.world, &session.assets);
                    }
                    let tick = session.world.tick;
                    let source_tick = scene.director.source.tick;
                    let update = playback.step(0).unwrap_or_else(|error| panic!("stage {stage}, source {character}, native {hero}, tick {source_tick}: {error}"));
                    for sound in update.original_sounds {
                        let bank = sound.bank.expect("Original cue lost its bank");
                        assert!(!bank.samples.is_empty());
                        sounds += 1;
                    }
                    if let Content::Gameplay(session) = &playback.content {
                        assert_eq!(session.presentation_tick(), source_tick + 1);
                        if session.cinematic() {
                            assert_eq!(session.world.tick, tick);
                        }
                    }
                }
                assert!(
                    matches!(playback.content, Content::Results(_)),
                    "stage {stage} source {character} native {hero}: no results"
                );
                sequences += scenes.len();
                eprintln!(
                    "Imported host stage {stage}, source {character}, native {hero}: {} complete scenes",
                    scenes.len()
                );
            }
        }
    }
    assert!(sequences >= 198);
    assert!(sounds > 100);
    eprintln!("{sequences} authored scenes and {sounds} original-bank cues reached native results");
}

#[test]
#[ignore = "requires both discs; initial death, original wait, native failure/retry and equipment preserve the imported source"]
fn imported_host_failure_retry_restores_original_script_and_native_equipment() {
    use tenchu_core::tenchu2::actor::Velocity;
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    for hero in 0..3 {
        let source =
            gaisen::Source::open(path.as_ref(), 0, i16::from(hero == 1), 2, Language::English)
                .unwrap();
        let mut playback = Playback::gaisen(
            crate::source::open_disc(t2.as_ref()).unwrap(),
            source,
            hero,
            2,
            7,
            None,
            6000,
        )
        .unwrap();
        assert!(matches!(playback.content, Content::Equipment(_)));
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        playback.action = Some(campaign::Action::Continue);
        for _ in 0..40 {
            playback.step(0).unwrap();
            if matches!(playback.content, Content::Gameplay(_)) {
                break;
            }
        }
        let Content::Gameplay(session) = &mut playback.content else {
            panic!("No equipped mission");
        };
        let items = session.world.actors[session.world.player]
            .actor
            .inventory
            .amounts();
        let player = &mut session.world.actors[session.world.player].actor;
        player.health = 0;
        let death = (0xb00..=0xb05)
            .find(|&action| {
                session
                    .assets
                    .resources
                    .registry
                    .clip(player.definition_kind(), action)
                    .is_some()
            })
            .unwrap();
        player
            .change_motion(death, Velocity::Stop, &session.assets.resources)
            .unwrap()
            .unwrap();
        player.motion.loops = 1;
        for _ in 0..500 {
            playback.step(0).unwrap();
            if matches!(playback.content, Content::Failed) {
                break;
            }
        }
        assert!(matches!(playback.content, Content::Failed));
        playback.action = Some(campaign::Action::Retry);
        playback.step(0).unwrap();
        assert!(matches!(playback.content, Content::Equipment(_)));
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        playback.action = Some(campaign::Action::Continue);
        for _ in 0..40 {
            playback.step(0).unwrap();
            if matches!(playback.content, Content::Gameplay(_)) {
                break;
            }
        }
        let Content::Gameplay(session) = &playback.content else {
            panic!("No retry");
        };
        assert_eq!(session.gaisen.as_ref().unwrap().director.source.tick, 0);
        assert_eq!(session.world.tick, 0);
        assert_eq!(
            session.world.actors[session.world.player]
                .actor
                .inventory
                .amounts(),
            items
        );
        assert_eq!(
            session.world.terrain.gaisen.as_ref().unwrap().water_depth,
            6000
        );
        assert_eq!(
            session
                .gaisen
                .as_ref()
                .unwrap()
                .director
                .source
                .player()
                .kind,
            i16::from(hero == 1)
        );
    }
}

#[test]
#[ignore = "requires both discs; unlimited cinematic acceleration stops at imported/native handoffs and preserves world/RNG"]
fn imported_host_acceleration_stops_at_native_gameplay() {
    use super::super::transport::{Input, Transport};
    use std::time::Duration;
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    let start = || {
        let source = gaisen::Source::open(path.as_ref(), 0, 0, 1, Language::English).unwrap();
        let mut playback = Playback::gaisen(
            crate::source::open_disc(t2.as_ref()).unwrap(),
            source,
            0,
            2,
            7,
            Some([0; 21]),
            4000,
        )
        .unwrap();
        playback.step(0).unwrap();
        let Content::Gameplay(session) = &mut playback.content else {
            unreachable!()
        };
        director::force_next(
            &session.gaisen.as_ref().unwrap().director,
            &mut session.world,
            &session.assets,
        );
        playback.step(0).unwrap();
        assert!(playback.content.cutscene());
        playback
    };
    let mut normal = start();
    for _ in 0..6000 {
        normal.step(0).unwrap();
        if !normal.content.cutscene() {
            break;
        }
    }
    assert!(!normal.content.cutscene());
    let mut fast = start();
    let mut transport = Transport::default();
    for _ in 0..6000 {
        let update = transport
            .advance(
                &mut fast,
                Duration::from_millis(34),
                Input {
                    buttons: 0,
                    cutscene_speed: 0,
                    simulation_speed: 100,
                    fast_forward: true,
                    paused: false,
                },
                None,
                |_, _| Ok(()),
            )
            .unwrap();
        if update.changed {
            break;
        }
    }
    assert!(!fast.content.cutscene());
    let (Content::Gameplay(a), Content::Gameplay(b)) = (&normal.content, &fast.content) else {
        panic!("Cinematic left imported mission");
    };
    assert_eq!(
        postcard::to_allocvec(&a.world).unwrap(),
        postcard::to_allocvec(&b.world).unwrap()
    );
    assert_eq!(
        postcard::to_allocvec(&a.gaisen.as_ref().unwrap().director).unwrap(),
        postcard::to_allocvec(&b.gaisen.as_ref().unwrap().director).unwrap()
    );
}
