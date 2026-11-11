use super::*;

fn playback(sequence: usize) -> Playback<crate::source::DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    Playback::load(disc, sequence, 2, 17, Some([0; 21]), None).unwrap()
}

#[test]
#[ignore = "requires TENCHU2_DISC; reports must preserve native gameplay/cutscene state and omit resource data"]
fn native_reports_preserve_source_state_and_reset_inputs_at_scene_boundaries() {
    let mut playback = playback(119);
    let mut reference = self::playback(119);
    let mut transport = Transport::default();
    let settings = Settings {
        model_directory: r"C:\Users\忍者\models".into(),
        ..Default::default()
    };
    let mut menu = Menu::new(&settings);
    menu.panel.notice = r"Reloading C:\Users\忍者\models\Ayame".into();
    let controls = Controls::default();
    for tick in 0..90 {
        let buttons = if tick < 15 {
            tenchu_core::tenchu2::input::UP
        } else {
            0
        };
        transport
            .frame_step(&mut playback, buttons, None, |_, _| Ok(()))
            .unwrap();
        reference.step(buttons).unwrap();
    }
    let Content::Gameplay(session) = &playback.content else {
        panic!("gameplay");
    };
    let Content::Gameplay(expected) = &reference.content else {
        panic!("gameplay");
    };
    let before = serde_json::to_vec(&session.world).unwrap();
    assert_eq!(before, serde_json::to_vec(&expected.world).unwrap());
    let snapshot = report(&playback, &transport, &menu, &settings, &controls);
    let bytes = crate::diagnostics::finish(snapshot, &settings, None).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(!text.contains("Users") && !text.contains("忍者"));
    assert!(bytes.len() < 512 * 1024);
    let parsed = crate::diagnostics::decode(&bytes).unwrap();
    assert_eq!(parsed["engine"], "tenchu2-jp");
    assert_eq!(
        parsed["game"]["random"],
        serde_json::to_value(session.world.random).unwrap()
    );
    assert_eq!(
        parsed["game"]["actors"][0]["health"],
        session.world.actors[0].actor.health
    );
    assert_eq!(
        parsed["recent_inputs"]["inputs"].as_array().unwrap().len(),
        90
    );
    assert_eq!(
        parsed["recent_inputs"]["inputs"][0]["buttons"],
        tenchu_core::tenchu2::input::UP
    );
    assert_eq!(parsed["runtime"]["notice"], r"Reloading <redacted>\Ayame");
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
    for resource in ["terrain", "scene", "assets", "bank", "vram", "progress"] {
        assert!(parsed["game"].get(resource).is_none());
        assert!(parsed["session"].get(resource).is_none());
    }
    assert!(filename(&playback).contains("t2jp-s119-tick90"));

    playback.action = Some(super::super::campaign::Action::SelectMission);
    transport
        .frame_step(&mut playback, 0, None, |_, _| Ok(()))
        .unwrap();
    assert!(transport.recent.inputs.is_empty());
    assert_eq!(
        report(&playback, &transport, &menu, &settings, &controls)["runtime"]["screen"],
        "mission-selection"
    );

    let mut story = self::playback(10);
    for _ in 0..10 {
        transport
            .frame_step(&mut story, 0, None, |_, _| Ok(()))
            .unwrap();
    }
    let Content::Story { story: content, .. } = &story.content else {
        panic!("story");
    };
    let before = serde_json::to_vec(&content.state).unwrap();
    let report = report(&story, &transport, &menu, &settings, &controls);
    assert_eq!(report["runtime"]["screen"], "story");
    assert!(report.get("game").is_none());
    assert_eq!(report["story"]["frame"], content.state.frame);
    assert_eq!(serde_json::to_vec(&content.state).unwrap(), before);
}
