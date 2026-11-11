use super::*;

#[test]
#[ignore = "requires both discs; cutscene checkpoints retain faces, blood, model overrides and interpolation across restores"]
fn native_story_checkpoints_preserve_faces_blood_and_overrides() {
    use crate::visuals::appearance::library::Library;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let bodies = std::sync::Arc::default();
    let campaign = tenchu_assets::tenchu2::campaign::Campaign::load(&disc).unwrap();
    for (event, frame, model) in [
        (201, 340, "disc:1"),
        (11, 620, "tenchu2-jp:ayamea"),
        (5, 305, "disc:0"),
        (111, 340, "disc:0"),
    ] {
        let sequence = campaign
            .sequences
            .iter()
            .position(|row| {
                row.kind == tenchu_assets::tenchu2::campaign::Kind::Event && row.resource == event
            })
            .unwrap();
        let hero = (0..3)
            .find(|&hero| campaign.mission(hero, sequence).is_some())
            .unwrap();
        let mut original = Story::load(&disc, sequence, event, 17).unwrap();
        original
            .models
            .sync(
                &original.resources,
                hero,
                &library,
                &bodies,
                &crate::settings::Settings {
                    player_model: model.into(),
                    ..Default::default()
                },
            )
            .unwrap();
        original.interpolate(true).unwrap();
        for _ in 0..=frame {
            original.step(true).unwrap();
        }
        if event == 11 {
            assert!(
                original
                    .state
                    .effects
                    .frame
                    .points
                    .iter()
                    .any(|p| p.blood.is_some())
            );
        }
        if event == 5 {
            assert!(
                original
                    .state
                    .effects
                    .frame
                    .quads
                    .iter()
                    .any(|q| q.blood.is_some())
            );
        }
        let bytes = original.capture_state().unwrap();
        // Restore backwards into an already animated scene with later facial uploads.
        let mut restored = Story::load(&disc, sequence, event, 17).unwrap();
        for _ in 0..frame + 90 {
            restored.step(true).unwrap();
        }
        restored.restore_state(&disc, &bytes).unwrap();
        assert!(bytes == restored.capture_state().unwrap());
        for tick in 0..180 {
            let draw = tick % 7 != 3;
            original.step(draw).unwrap();
            restored.step(draw).unwrap();
            assert!(
                postcard::to_allocvec(&original.state).unwrap()
                    == postcard::to_allocvec(&restored.state).unwrap(),
                "event {event}, update {tick}"
            );
            assert!(
                postcard::to_allocvec(&(&original.contact, &original.interpolation)).unwrap()
                    == postcard::to_allocvec(&(&restored.contact, &restored.interpolation))
                        .unwrap()
            );
            for (&id, model) in &original.resources.models {
                assert!(model.vram.words == restored.resources.models[&id].vram.words);
            }
        }
        assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
        original
            .models
            .sync(
                &original.resources,
                hero,
                &library,
                &bodies,
                &crate::settings::Settings::default(),
            )
            .unwrap();
        restored
            .models
            .sync(
                &restored.resources,
                hero,
                &library,
                &bodies,
                &crate::settings::Settings::default(),
            )
            .unwrap();
        assert!(original.models.actors.is_empty() && restored.models.actors.is_empty());
        assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
        eprintln!(
            "Story {event}: {} compressed bytes; 180 updates and facial atlases match",
            bytes.len()
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; invalid story identities, textures and motion references reject transactionally"]
fn native_story_checkpoints_reject_invalid_state() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut story = Story::load(&disc, 0, 201, 17).unwrap();
    for _ in 0..90 {
        story.step(true).unwrap();
    }
    let original = story.capture_state().unwrap();
    for change in [
        (|s: &mut Owned| s.signature.clear()) as fn(&mut Owned),
        |s| s.event = 11,
        |s| s.playback.frame = i16::MAX,
        |s| {
            s.playback.actors.pop();
        },
        |s| s.playback.camera.rotation.clear(),
        |s| s.textures.values_mut().next().unwrap().words.clear(),
        |s| {
            s.textures.pop_first();
        },
        |s| s.playback.faces.actors[0].model = i16::MAX,
        |s| {
            s.models.actors.insert(i16::MAX);
        },
    ] {
        let mut saved: Owned = crate::savestate::decode(&original).unwrap();
        change(&mut saved);
        assert!(
            story
                .restore_state(&disc, &crate::savestate::encode(&saved).unwrap())
                .is_err()
        );
        assert!(story.capture_state().unwrap() == original);
    }
}
