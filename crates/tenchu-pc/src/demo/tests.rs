use super::*;

fn player(args: &[&str]) -> Player {
    Player::parse(&args.iter().map(OsString::from).collect::<Vec<_>>()).unwrap()
}

#[test]
fn timeline_and_speed_changes_do_not_depend_on_display_cadence() {
    for id in ["moon", "motion", "speed", "cinematics", "step"] {
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
    for id in ["moon", "motion", "speed", "cinematics", "step"] {
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
