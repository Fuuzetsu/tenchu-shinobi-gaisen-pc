use super::*;

fn player(args: &[&str]) -> Player {
    Player::parse(&args.iter().map(OsString::from).collect::<Vec<_>>()).unwrap()
}

#[test]
fn timeline_and_speed_changes_do_not_depend_on_display_cadence() {
    for id in [
        "moon",
        "motion",
        "speed",
        "cinematics",
        "step",
        "snow-footprints",
        "experiment",
    ] {
        let run = |fps: u64| {
            let mut p = player(&["--clips", id]);
            let mut samples = Vec::new();
            for frame in 0..fps * 10 {
                p.elapse(Duration::from_nanos(
                    (frame + 1) * 1_000_000_000 / fps - frame * 1_000_000_000 / fps,
                ));
                while let Some(ticks) = p.step(true) {
                    samples.push((p.frame, ticks, p.simulation_ticks));
                }
            }
            (samples, p.alpha())
        };
        let expected = run(60);
        for fps in [24, 30, 75, 144, 240] {
            assert_eq!(run(fps), expected, "{id} at {fps}fps");
        }
        if id == "step" {
            assert_eq!(expected.0.last().unwrap().2, 20);
        }
        if id == "moon" {
            assert_eq!(expected.0.last().unwrap().2, 300);
        }
    }
}

#[test]
fn playlist_order_resets_and_end_behavior_are_explicit() {
    let mut p = player(&["--clips", "moon,shadows,moon", "--duration", "2", "--clean"]);
    assert_eq!(p.playlist.len(), 3);
    assert!(!p.captions && !p.transport);
    p.elapse(Duration::from_secs(2));
    while p.step(true).is_some() {}
    assert!(p.ended());
    assert!(p.next());
    assert_eq!(p.clip().id, "shadows");
    assert_eq!(p.simulation_ticks, 0);
    assert_eq!(p.alpha(), 0.0);
    assert!(p.next());
    assert!(!p.next());
    assert!(p.finished);
    p.repeat = true;
    assert!(p.next());
    assert_eq!(p.index, 0);
    assert!(!p.finished);
    for args in [
        vec!["--clips", "missing"],
        vec!["--duration", "0"],
        vec!["--loop", "--exit"],
        vec!["--clips", ""],
        vec!["--video", "out.mp4", "--loop"],
        vec!["--video"],
    ] {
        assert!(Player::parse(&args.into_iter().map(OsString::from).collect::<Vec<_>>()).is_err());
    }
}

#[test]
fn offline_frames_match_the_live_timeline_without_rounding_or_wall_clock() {
    for id in [
        "moon",
        "motion",
        "speed",
        "cinematics",
        "step",
        "snow-footprints",
        "experiment",
    ] {
        let mut live = player(&["--clips", id]);
        let mut offline = player(&["--clips", id, "--video", "out.mp4"]);
        assert!(!offline.transport && offline.captions && offline.muted);
        for frame in 0..600 {
            live.elapse(Duration::from_nanos(
                ((frame + 1) * 1_000_000_000_u64).div_ceil(60)
                    - (frame * 1_000_000_000_u64).div_ceil(60),
            ));
            offline.queue_frame();
            assert_eq!(offline.step(true), live.step(true));
            assert!(offline.step(true).is_none());
            assert!(live.step(true).is_none());
            assert_eq!(offline.simulation_ticks, live.simulation_ticks);
            assert!((offline.alpha() - live.alpha()).abs() < 1e-5);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; both halves of the item-light demos must produce real item effects"]
fn item_light_comparisons_use_working_items_on_both_sides() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for (id, light_name) in [("grenade-light", "Explosion"), ("kaen-light", "Kaen flame")] {
        let mut p = player(&["--clips", id]);
        let (mut game, _) = p.load_game(&assets, None).unwrap();
        let mut seen = [false; 2];
        let ticks = u64::from(p.clip().seconds) * 30;
        let lighting = crate::settings::Lighting {
            enabled: true,
            lamps: false,
            ..Default::default()
        };
        for tick in 0..ticks {
            game.step(p.buttons(tick)).unwrap();
            let sources = crate::lighting::sources(&game, &lighting, None, None, Vec::new());
            seen[usize::from(tick >= ticks / 2)] |= sources.iter().any(|s| s.name == light_name);
        }
        assert_eq!(
            seen, [true; 2],
            "{id}: an item failed to activate in one comparison"
        );
    }
}

#[test]
fn catalog_rejects_duplicate_ids_and_invalid_scene_pairings() {
    let mut json: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../data/demos.json")).unwrap();
    let clips = json.as_array_mut().unwrap();
    clips[1]["id"] = clips[0]["id"].clone();
    assert!(catalog::parse(&serde_json::to_vec(&json).unwrap()).is_err());
    let mut json: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../data/demos.json")).unwrap();
    json[0]["shot"] = "cinematic".into();
    assert!(catalog::parse(&serde_json::to_vec(&json).unwrap()).is_err());
    json[0]["shot"] = "wide".into();
    json[0]["feature"] = "artwork".into();
    json[0]["frontend"] = serde_json::json!({"page":"missions", "language":"ja"});
    assert!(catalog::parse(&serde_json::to_vec(&json).unwrap()).is_ok());
    json[0]["mission"] = 0.into();
    assert!(catalog::parse(&serde_json::to_vec(&json).unwrap()).is_err());
    json[0]["mission"] = 1.into();
    json[0]["frontend"]["language"] = "unknown".into();
    assert!(catalog::parse(&serde_json::to_vec(&json).unwrap()).is_err());
    assert_eq!(sweep(0.0), 0.0);
    assert_eq!(sweep(0.5), 1.0);
    assert_eq!(sweep(1.0), 0.0);
}

#[test]
fn cinematic_acceleration_does_not_spill_into_gameplay() {
    let mut p = player(&["--clips", "cinematics", "--duration", "2"]);
    p.elapse(Duration::from_secs(2));
    while p.step(false).is_some() {}
    assert_eq!(p.simulation_ticks, 60);
}

#[test]
fn authored_camera_paths_reject_crossings_and_keep_the_target_centered() {
    let mut path: framing::Path = serde_json::from_str(
        r#"{
        "eye":[1000,-2000,5000], "target":[0,0,0],
        "end_eye":[4000,-1000,2000], "end_target":[500,0,500]
    }"#,
    )
    .unwrap();
    path.validate().unwrap();
    for step in 0..=60 {
        let progress = step as f32 / 60.0;
        let t = progress * progress * (3.0 - 2.0 * progress);
        let target =
            glam::Vec3::from(path.target).lerp(glam::Vec3::from(path.end_target.unwrap()), t);
        let center = path
            .sample(progress)
            .matrix(16.0 / 9.0)
            .project_point3(target);
        assert!(center.x.abs() < 1e-5 && center.y.abs() < 1e-5);
    }
    path.end_eye = Some([-1000.0, 2000.0, -5000.0]);
    path.end_target = None;
    assert!(path.validate().is_err());
}

#[test]
#[ignore = "requires TENCHU_DISC; recorded demo excerpts preserve the original seed, inventory, events and input sequence"]
fn recorded_excerpt_matches_direct_replay_before_and_after_artwork_toggle() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap())
            .unwrap();
    let input_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/mission1-rikimaru.inputs");
    let text = std::fs::read_to_string(&input_path).unwrap();
    let start = crate::recording::Start::parse(&text).unwrap().unwrap();
    let mut expected = start.load(&assets).unwrap();
    let runs = crate::session::parse_inputs(&text).unwrap();
    let mut inputs = runs
        .into_iter()
        .flat_map(|(count, buttons)| std::iter::repeat_n(buttons, count as usize));
    for buttons in inputs.by_ref().take(200) {
        expected.step(buttons).unwrap();
    }
    let initial = crate::session::tests::state(&expected);
    let mut p = player(&["--clips", "artwork", "--duration", "2"]);
    let index = p.playlist[0];
    p.catalog[index].mission = 1;
    p.catalog[index].character = 0;
    p.catalog[index].shot = Shot::Gameplay;
    p.catalog[index].recording = Some(catalog::Recording {
        path: input_path,
        start_tick: 200,
    });
    let buttons: Vec<_> = inputs.take(60).collect();
    for &buttons in &buttons {
        expected.step(buttons).unwrap();
    }
    let final_state = crate::session::tests::state(&expected);
    for artwork in [false, true] {
        p.reset(0);
        p.catalog[index].artwork = Some(artwork);
        let (mut actual, _) = p.load_game(&assets, None).unwrap();
        assert_eq!(crate::session::tests::state(&actual), initial);
        let mut settings = crate::settings::Settings::default();
        let mut debug = crate::debug::Debug::new(&actual).unwrap();
        for _ in 0..120 {
            p.queue_frame();
            let ticks = p.step(false).unwrap();
            animate(&p, &mut settings, &mut debug);
            assert_eq!(settings.graphics.asset_pack, artwork);
            for tick in p.simulation_ticks - ticks as u64..p.simulation_ticks {
                assert_eq!(p.buttons(tick), buttons[tick as usize]);
                actual.step(p.buttons(tick)).unwrap();
            }
        }
        assert_eq!(crate::session::tests::state(&actual), final_state);
    }
    p.catalog[index].recording.as_mut().unwrap().start_tick = u32::MAX;
    assert!(p.load_game(&assets, None).is_err());
}
