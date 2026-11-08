use serde::Deserialize;
use std::collections::BTreeMap;
use tenchu_assets::tenchu2::{
    campaign::{Campaign, Kind},
    motions,
    story::Script,
};
use tenchu_core::tenchu2::story::Motion;

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_EXIT_ORACLE; original cutscene final-frame/skip/letterbox/fade transitions"]
fn native_story_exit_matches_original_outer_loop() {
    use tenchu_assets::tenchu2::{
        campaign::{Objective, Region, Sequence},
        story::Resources,
    };
    use tenchu_core::tenchu2::story::{Fade, Playback};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = Resources::load(&disc, 4).unwrap();
    resources.script.duration = 5;
    resources.script.commands.clear();
    resources.script.expressions.clear();
    let basis = motions::spline_basis(&disc).unwrap();
    let math = tenchu_core::math::Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let reach = Kind::Gameplay(Objective::Reach {
        region: Region {
            center: [0; 3],
            radius: 0,
        },
        unseen: false,
    });
    let mut checked = 0;
    for (case, row) in
        super::source_control::cases::<serde_json::Value>("TENCHU2_STORY_EXIT_ORACLE").enumerate()
    {
        let mut state = Playback::new(&resources, 17);
        state.presentation.fade = row["fade"].as_bool().unwrap().then_some(Fade {
            white: true,
            outward: true,
            duration: 100,
            age: 0,
        });
        let button = row["button"].as_u64().unwrap() as u16;
        let kind = match row["next_kind"].as_i64().unwrap() {
            -1 => None,
            0 => Some(Kind::Introduction),
            0x10 => Some(if button == 0x4000 { reach } else { Kind::Event }),
            0x20 => Some(Kind::Movie),
            0x31 => Some(reach),
            0x33 => Some(Kind::Gameplay(Objective::Weaken { targets: [0; 3] })),
            0x34 => Some(Kind::Gameplay(Objective::Defeat { targets: [0; 3] })),
            0x40 => Some(Kind::Results),
            _ => unreachable!(),
        };
        let next = kind.map(|kind| Sequence {
            kind,
            resource: 2,
            next: Some(2),
            failure: None,
        });
        let states = row["states"].as_array().unwrap();
        for (tick, expected) in states.iter().enumerate() {
            assert!(
                !state.ending.is_some_and(|e| e.complete),
                "ended early: {case}:{tick}"
            );
            state.step(&resources.script, &basis, &math, true).unwrap();
            if state.ending.is_none()
                && (state.finished(&resources.script) || (button != 0 && tick == 2))
            {
                state.begin_exit(next, button == 0x4000, resources.results_fade);
            }
            assert_eq!(
                serde_json::json!({
                    "frame": state.frame,
                    "exit_age": state.ending.map_or(0, |e| e.age),
                    "fade": state.presentation.fade,
                    "offset": state.overlay.letterbox_offset,
                    "overlays": state.overlay.overlays.iter().map(|o| o.rgb[0]).collect::<Vec<_>>(),
                }),
                *expected,
                "exit case {case}, tick {tick}"
            );
            if tick == 8 {
                state = serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
            }
            checked += 1;
        }
        assert!(state.ending.unwrap().complete, "did not finish: {case}");
    }
    assert!(checked > 500);
    eprintln!("{checked} original JP cutscene exit draws");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_FACES_ORACLE; original expression/blink/lip timing and RNG"]
fn native_story_faces_match_original_expression_and_draw_updates() {
    use tenchu_assets::tenchu2::story::Resources;
    use tenchu_core::{
        random::Random,
        tenchu2::story::{Face, Faces, TextureChange},
    };
    #[derive(Deserialize)]
    struct State {
        frame: i16,
        draw: bool,
        changes: Vec<TextureChange>,
        actors: Vec<Face>,
        seed: u32,
    }
    #[derive(Deserialize)]
    struct Case {
        event: u16,
        seed: u32,
        states: Vec<State>,
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut checked = 0;
    for case in super::source_control::cases::<Case>("TENCHU2_STORY_FACES_ORACLE") {
        let mut resources = Resources::load(&disc, case.event).unwrap();
        let mut faces = Faces::new(&resources);
        let mut random = Random::new(case.seed);
        for state in case.states {
            let changes = faces.update(
                &resources.script.expressions,
                state.frame,
                state.draw,
                &mut random,
            );
            assert_eq!(
                changes, state.changes,
                "event {} frame {}",
                case.event, state.frame
            );
            assert_eq!(
                faces.actors, state.actors,
                "event {} frame {}",
                case.event, state.frame
            );
            assert_eq!(
                random.seed, state.seed,
                "event {} frame {}",
                case.event, state.frame
            );
            for change in changes {
                if let TextureChange::Patch { model, index } = change {
                    resources.faces[&model][usize::from(index)]
                        .apply(&mut resources.models.get_mut(&model).unwrap().vram);
                }
            }
            if state.frame % 101 == 0 {
                faces = serde_json::from_value(serde_json::to_value(&faces).unwrap()).unwrap();
            }
            checked += 1;
        }
    }
    assert!(checked > 8000);
    eprintln!("{checked} original JP facial animation updates, including draw skips");
}

#[test]
#[ignore = "requires TENCHU2_DISC; loads every JP story cast, scenery model and texture group"]
fn native_story_models_load_from_their_event_bundles() {
    use tenchu_assets::tenchu2::story::Resources;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut pending: Vec<_> = campaign
        .starts
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
    let mut visited = std::collections::BTreeSet::new();
    let mut events = std::collections::BTreeSet::new();
    while let Some(index) = pending.pop() {
        if !visited.insert(index) {
            continue;
        }
        let sequence = campaign.sequences[index];
        if sequence.kind == Kind::Event {
            events.insert(sequence.resource);
        }
        pending.extend(sequence.next);
        pending.extend(sequence.failure);
    }
    // Event 88 has an unused sequence descriptor and no E88.ARC on the JP disc.
    assert_eq!(events.len(), 141);
    let mut models = 0;
    let mut weapons = 0;
    let mut captions = 0;
    let mut voices = 0;
    let mut characters = std::collections::BTreeSet::new();
    for event in events {
        let resources =
            Resources::load(&disc, event).unwrap_or_else(|e| panic!("event {event}: {e}"));
        for actor in &resources.script.actors {
            if let Some(model) = resources.models.get(&actor.model) {
                tenchu_core::motion::Pose::new(&model.parts).unwrap();
                assert!(!model.parts.is_empty(), "event {event}: {}", model.name);
                assert!(
                    model.vram.words.iter().any(|&v| v != 0),
                    "event {event}: {} has no texture data",
                    model.name
                );
                models += 1;
            } else {
                assert!(resources.effects.contains_key(&actor.model));
            }
        }
        for model in resources.weapons.values() {
            tenchu_core::motion::Pose::new(&model.parts).unwrap();
        }
        if let Some(track) = &resources.audio {
            let mut stream = tenchu_assets::audio::stream::Stream::open(&disc, track).unwrap();
            assert!(
                stream.next(&disc).unwrap().is_some(),
                "event {event}: empty XA cue"
            );
            voices += 1;
        }
        for command in &resources.script.commands {
            if let tenchu_assets::tenchu2::story::Command::Caption { text, .. } = &command.command {
                captions += 1;
                characters.extend(text.chars().filter(|c| !c.is_control()));
            }
        }
        for picture in resources.portraits.iter().chain(&resources.pictures) {
            assert_eq!(
                picture.rgba.len(),
                usize::from(picture.size[0]) * usize::from(picture.size[1]) * 4
            );
            assert!(
                picture.rgba.chunks_exact(4).any(|p| p[3] != 0),
                "event {event}: empty image"
            );
        }
        weapons += resources.attachments.len();
    }
    assert!(models > 500);
    assert!(weapons > 100);
    assert!(captions > 1000);
    assert!(voices > 100);
    let context = egui::Context::default();
    crate::fonts::install(&context);
    let _ = context.run(Default::default(), |context| {
        context.fonts(|fonts| {
            for character in &characters {
                if *character != '?' {
                    assert!(
                        fonts.has_glyph(&crate::fonts::id(14.0), *character),
                        "Missing JP story character {character:?}"
                    );
                }
            }
        });
    });
    eprintln!(
        "{models} native JP story model placements, {weapons} weapon attachments, {captions} captions and {voices} playable XA cues loaded"
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_MOTION_ORACLE; source event root/joint tracks"]
fn native_story_tracks_match_original_event_animations() {
    #[derive(Deserialize)]
    struct State {
        frame: i16,
        position: [i32; 3],
        rotation: Vec<[i16; 3]>,
    }
    #[derive(Deserialize)]
    struct Case {
        event: u16,
        slot: usize,
        states: Vec<State>,
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let basis = motions::spline_basis(&disc).unwrap();
    let mut scripts = BTreeMap::new();
    for sequence in campaign.sequences {
        if sequence.kind == Kind::Event {
            scripts.entry(sequence.resource).or_insert_with(|| {
                Script::load(&disc, sequence.resource)
                    .unwrap_or_else(|e| panic!("event {}: {e}", sequence.resource))
            });
        }
    }
    assert_eq!(scripts.len(), 142);
    let mut checked = 0;
    for case in super::source_control::cases::<Case>("TENCHU2_STORY_MOTION_ORACLE") {
        let script = scripts
            .entry(case.event)
            .or_insert_with(|| Script::load(&disc, case.event).unwrap());
        let animation = if case.slot == 0 {
            &script.camera
        } else {
            &script.actors[case.slot - 1]
        };
        let mut motion = Motion::new(animation);
        for expected in case.states {
            motion.update(animation, expected.frame, &basis).unwrap();
            assert_eq!(
                motion.position, expected.position,
                "event {} slot {} frame {} position",
                case.event, case.slot, expected.frame
            );
            assert_eq!(
                motion.rotation, expected.rotation,
                "event {} slot {} frame {} rotation",
                case.event, case.slot, expected.frame
            );
            motion = serde_json::from_value(serde_json::to_value(&motion).unwrap()).unwrap();
            checked += 1;
        }
    }
    assert!(checked > 3000);
    eprintln!("{checked} original JP event animation states; all 142 campaign events parsed");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_OVERLAYS_ORACLE; original command/overlay timing"]
fn native_story_presentation_matches_original_timers_and_draw_skips() {
    use tenchu_core::tenchu2::story::Presentation;
    #[derive(Deserialize)]
    struct Case {
        event: u16,
        states: Vec<serde_json::Value>,
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut checked = 0;
    for case in super::source_control::cases::<Case>("TENCHU2_STORY_OVERLAYS_ORACLE") {
        let script = Script::load(&disc, case.event).unwrap();
        let mut presentation = Presentation::default();
        for expected in case.states {
            let mut expected = expected.as_object().unwrap().clone();
            let frame = expected.remove("frame").unwrap().as_i64().unwrap() as i16;
            let draw = expected.remove("draw").unwrap().as_bool().unwrap();
            let output = presentation.update(&script, frame, draw);
            let actual = serde_json::json!({
                "fade": presentation.fade,
                "tint": presentation.tint,
                "warp": presentation.warp,
                "portraits": presentation.portraits,
                "picture": presentation.picture,
                "remaining": presentation.caption.map_or(0, |c| c.remaining),
                "caption_shown": output.caption.is_some(),
                "picture_shown": output.picture,
                "camera_roll": presentation.camera_roll,
                "weather": presentation.weather,
                "overlays": output.overlays,
            });
            assert_eq!(
                actual,
                serde_json::Value::Object(expected),
                "event {} frame {frame}",
                case.event
            );
            if frame % 37 == 0 {
                presentation =
                    serde_json::from_value(serde_json::to_value(&presentation).unwrap()).unwrap();
            }
            checked += 1;
        }
    }
    assert!(checked > 10_000);
    eprintln!("{checked} original JP command/overlay updates, including draw skips");
}
