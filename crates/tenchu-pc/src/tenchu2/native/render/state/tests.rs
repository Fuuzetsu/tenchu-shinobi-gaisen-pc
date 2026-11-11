use super::*;
use crate::{
    settings::{Blood, Motion, WeatherMode},
    tenchu2::native::{Session, story::Story},
};

fn graphics() -> Graphics {
    let mut graphics = Graphics {
        blood: Blood::modern(),
        asset_pack: false,
        motion: Motion {
            hair: true,
            cloth: true,
            equipment: true,
            wind: 100,
            ..Default::default()
        },
        ..Default::default()
    };
    graphics.weather.mode = WeatherMode::Snow;
    graphics.sky.enabled = true;
    graphics.blood.spurts = 100;
    graphics
}

fn equal(a: &ContentDrawing, b: &ContentDrawing) {
    assert!(
        postcard::to_allocvec(&a.capture_state()).unwrap()
            == postcard::to_allocvec(&b.capture_state()).unwrap(),
        "restored presentation did not continue identically"
    );
}

fn record(history: &mut crate::rewind::History, state: State, tick: u64) {
    history.recorder.begin();
    state.capture(&mut history.recorder).unwrap();
    let snapshot = history.recorder.finish().unwrap();
    assert!(snapshot.bytes().unwrap() == postcard::to_allocvec(&state).unwrap());
    history.push(tick, 30, snapshot, None).unwrap();
}

fn output() -> std::path::PathBuf {
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    out
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; active wounds, weather and secondary motion survive GPU reconstruction and continue exactly"]
fn native_drawing_checkpoints_preserve_wounds_weather_and_motion() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = output();
    let graphics = graphics();
    for hero in 0..3 {
        let mut images = Vec::new();
        for restored_image in [false, true] {
            let (mut session, player, guard) = wounds::tests::combat_fixture(hero);
            let vram = session.assets.sprites.vram.clone();
            let path = out.join(format!("hero-{hero}-restored-{restored_image}.png"));
            capture_frame(&vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                for _ in 0..12 {
                    session.step_world(0, true)?;
                    drawing.update(&session, renderer, None)?;
                }
                wounds::tests::weapon_hit(&mut session, guard, player)?;
                drawing.tick(&session, &graphics)?;
                let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                assert!(blood.has_drops());
                assert!(blood.attached_parts().next().is_some());
                assert!(
                    drawing
                        .motion
                        .sample(1.0)
                        .values()
                        .any(|pose| pose.offsets.iter().any(|o| o.length() > 0.01))
                );
                for actor in &mut session.world.actors {
                    actor.actor.invulnerable = true;
                }
                let bytes = session.capture_state()?;
                let mut restored = Session::load(&disc, 1, 1)?;
                restored.restore_state(&bytes)?;
                let original = Content::Gameplay(Box::new(session));
                let restored = Content::Gameplay(Box::new(restored));
                let mut a = ContentDrawing::Gameplay(Box::new(drawing));
                let mut history = crate::rewind::History::default();
                let saved = crate::savestate::encode(&a.capture_state().unwrap())?;
                record(&mut history, a.capture_state().unwrap(), 0);
                renderer.reset_vram(&vram);
                let mut b = ContentDrawing::new(&restored, &graphics, renderer)?;
                b.restore_state(crate::savestate::decode(&saved)?, &restored, renderer)?;
                equal(&a, &b);
                let Content::Gameplay(mut original) = original else {
                    unreachable!()
                };
                let Content::Gameplay(mut restored) = restored else {
                    unreachable!()
                };
                for tick in 0..=90 {
                    if tick != 0 {
                        original.step_world(0, true)?;
                        restored.step_world(0, true)?;
                    }
                    let (ContentDrawing::Gameplay(a), ContentDrawing::Gameplay(b)) =
                        (&mut a, &mut b)
                    else {
                        unreachable!()
                    };
                    for alpha in [0.25, 1.0] {
                        a.alpha = alpha;
                        b.alpha = alpha;
                        a.update(&original, renderer, None)?;
                        b.update(&restored, renderer, None)?;
                        assert!(
                            postcard::to_allocvec(&a.capture_state()).unwrap()
                                == postcard::to_allocvec(&b.capture_state()).unwrap(),
                            "hero {hero}, tick {tick}, alpha {alpha}"
                        );
                    }
                    record(&mut history, a.capture_state(), tick + 1);
                    assert!(
                        postcard::to_allocvec(&original.world).unwrap()
                            == postcard::to_allocvec(&restored.world).unwrap()
                    );
                    let x = a.blood_audio(&original)?;
                    let y = b.blood_audio(&restored)?;
                    assert_eq!(
                        x.iter().map(|s| s.sound).collect::<Vec<_>>(),
                        y.iter().map(|s| s.sound).collect::<Vec<_>>()
                    );
                }
                let state: State = history.snapshot(0)?.decode()?;
                assert!(crate::savestate::encode(&state)? == saved);
                let ContentDrawing::Gameplay(drawing) = (if restored_image { b } else { a }) else {
                    unreachable!()
                };
                Ok((drawing.scene, original.world.camera.view, drawing.texts))
            })
            .unwrap();
            images.push(
                crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                    .unwrap()
                    .rgba,
            );
        }
        assert!(images[0] == images[1], "hero {hero} restored pixels differ");
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; backwards story restores retain blood and resume motion on rebuilt models"]
fn native_drawing_checkpoints_preserve_story_stains_and_motion() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = output();
    let graphics = graphics();
    let mut images = Vec::new();
    for restored_image in [false, true] {
        let mut story = Story::load(&disc, 23, 11, 17).unwrap();
        for _ in 0..=600 {
            story.step(true).unwrap();
        }
        let vram = story.sprites.vram.clone();
        let path = out.join(format!("story-restored-{restored_image}.png"));
        capture_frame(&vram, &graphics, &path, |renderer| {
            let mut drawing = StoryDrawing::new(&story, &graphics, renderer)?;
            for _ in 0..50 {
                story.step(true)?;
                drawing.update(&story, renderer, None)?;
            }
            assert!(
                drawing
                    .scene
                    .environment
                    .as_ref()
                    .unwrap()
                    .blood
                    .pages()
                    .next()
                    .is_some()
            );
            let saved = story.capture_state()?;
            let mut restored = Story::load(&disc, 23, 11, 1)?;
            for _ in 0..900 {
                restored.step(true)?;
            }
            restored.restore_state(&disc, &saved)?;
            let a = ContentDrawing::Story(Box::new(drawing));
            let mut history = crate::rewind::History::default();
            let saved = crate::savestate::encode(&a.capture_state().unwrap())?;
            record(&mut history, a.capture_state().unwrap(), 0);
            let restored = Content::Story {
                story: Box::new(restored),
                audio: None,
            };
            renderer.reset_vram(&vram);
            let mut b = ContentDrawing::new(&restored, &graphics, renderer)?;
            b.restore_state(crate::savestate::decode(&saved)?, &restored, renderer)?;
            equal(&a, &b);
            let ContentDrawing::Story(mut a) = a else {
                unreachable!()
            };
            let ContentDrawing::Story(mut b) = b else {
                unreachable!()
            };
            let Content::Story {
                story: mut restored,
                ..
            } = restored
            else {
                unreachable!()
            };
            for tick in 0..=90 {
                if tick != 0 {
                    story.step(true)?;
                    restored.step(true)?;
                }
                for alpha in [0.25, 1.0] {
                    a.alpha = alpha;
                    b.alpha = alpha;
                    a.update(&story, renderer, None)?;
                    b.update(&restored, renderer, None)?;
                    assert!(
                        postcard::to_allocvec(&a.capture_state()).unwrap()
                            == postcard::to_allocvec(&b.capture_state()).unwrap(),
                        "story tick {tick}"
                    );
                }
                record(&mut history, a.capture_state(), tick + 1);
                assert!(
                    postcard::to_allocvec(&story.state).unwrap()
                        == postcard::to_allocvec(&restored.state).unwrap()
                );
            }
            let state: State = history.snapshot(0)?.decode()?;
            assert!(crate::savestate::encode(&state)? == saved);
            let drawing = if restored_image { b } else { a };
            Ok((drawing.scene, story.state.view, drawing.texts))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    assert!(images[0] == images[1], "restored story pixels differ");
}
