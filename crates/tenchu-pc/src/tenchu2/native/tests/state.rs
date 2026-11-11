use super::*;

fn assert_continuation(original: &mut Session, restored: &mut Session, ticks: usize) {
    for session in [&mut *original, &mut *restored] {
        if let Some(scene) = &mut session.gaisen {
            scene.audio.sounds.clear();
        }
    }
    for tick in 0..ticks {
        let buttons = match tick % 90 {
            0..30 => RIGHT,
            30..60 => UP,
            60 => ATTACK,
            _ => 0,
        };
        let a = original.step(buttons).unwrap();
        let b = restored.step(buttons).unwrap();
        let sounds = |sounds: Vec<Sourced<Sound>>| {
            postcard::to_allocvec(
                &sounds
                    .into_iter()
                    .map(|s| (s.sound, s.bank))
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        assert!(sounds(a) == sounds(b), "sound/voice bank at update {tick}");
        if let (Some(a), Some(b)) = (&mut original.gaisen, &mut restored.gaisen) {
            let sounds = |scene: &mut crate::tenchu2::native::gaisen::Scene| {
                postcard::to_allocvec(
                    &std::mem::take(&mut scene.audio.sounds)
                        .into_iter()
                        .map(|s| (s.sound.id, s.sound.volume, s.sound.pan, s.bank))
                        .collect::<Vec<_>>(),
                )
                .unwrap()
            };
            assert!(
                sounds(a) == sounds(b),
                "imported cinematic audio at update {tick}"
            );
        }
        assert!(
            postcard::to_allocvec(&original.world).unwrap()
                == postcard::to_allocvec(&restored.world).unwrap(),
            "world at update {tick}",
        );
        assert!(
            postcard::to_allocvec(&original.frame).unwrap()
                == postcard::to_allocvec(&restored.frame).unwrap(),
            "display at update {tick}",
        );
        assert!(
            postcard::to_allocvec(&(&original.contact, &original.interpolation)).unwrap()
                == postcard::to_allocvec(&(&restored.contact, &restored.interpolation)).unwrap(),
            "contacts/interpolation at update {tick}",
        );
    }
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
}

#[test]
#[ignore = "requires TENCHU2_DISC; complete session metadata and resources restore across areas for all heroes"]
fn native_session_snapshots_continue_across_areas() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [9, 119, 191] {
        let mut original = Session::load(&disc, sequence, 17).unwrap();
        original.world.actors[original.world.player]
            .actor
            .invulnerable = true;
        original.interpolate(true).unwrap();
        for _ in 0..60 {
            original.step(0).unwrap();
        }
        let position = original.world.actors[original.world.player].actor.position;
        original
            .world
            .effects
            .explosion(
                position,
                0,
                &original.assets.resources.math,
                &mut original.world.random,
            )
            .unwrap();
        original.step(ATTACK).unwrap();
        original.step(0).unwrap();
        assert!(!original.frame.effects.sprites.is_empty());
        let before =
            postcard::to_allocvec(&(&original.world, &original.assets, &original.frame)).unwrap();
        let start = std::time::Instant::now();
        let bytes = original.capture_state().unwrap();
        let elapsed = start.elapsed();
        assert!(
            before
                == postcard::to_allocvec(&(&original.world, &original.assets, &original.frame))
                    .unwrap()
        );
        let mut restored = Session::load(&disc, 1, 123).unwrap();
        restored.restore_state(&bytes).unwrap();
        assert!(bytes == restored.capture_state().unwrap());
        assert_continuation(&mut original, &mut restored, 180);
        eprintln!(
            "Session {sequence}: {} compressed bytes, capture {elapsed:?}; 180 updates match",
            bytes.len()
        );
    }
}

#[test]
#[ignore = "requires both discs; player overrides and mixed NPC voices survive mid-attack save, continuation and resetting models"]
fn native_session_snapshots_preserve_overrides_and_spawned_voices() {
    use crate::tenchu2::native::{models, spawning};
    use crate::{settings::Settings, visuals::appearance::library::Library};
    use tenchu_core::math::Position;
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = models::donor(&game).unwrap();
    let bodies = Arc::default();
    let mut original = Session::load(&disc, 9, 17).unwrap();
    original.world.actors[original.world.player]
        .actor
        .invulnerable = true;
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let position = original.world.actors[original.world.player].actor.position;
    for (offset, choice) in ["disc:16", "tenchu2-jp:tora", "tenchu2-jp:kagami"]
        .into_iter()
        .enumerate()
    {
        let id = original
            .spawn_actor(
                &disc,
                spawning::Request {
                    source: spawning::Source::Model(choice.into()),
                    position: position.wrapping_add(Position::new(
                        2000 + offset as i32 * 1000,
                        0,
                        0,
                    )),
                    yaw: 0,
                    snap: true,
                },
                Some(&library),
            )
            .unwrap();
        if offset == 1 {
            original.actor_voices.insert(id, None);
        }
    }
    assert!(original.actor_voices.values().any(Option::is_none));
    assert!(original.actor_voices.values().any(Option::is_some));
    let settings = Settings {
        player_model: "disc:1".into(),
        model_moveset: true,
        moveset_model: "tenchu2-jp:ayamea".into(),
        weapon_model: "tenchu2-jp:yorojyu".into(),
        voice_model: "tenchu2-jp:kagami".into(),
        ..Default::default()
    };
    assert!(
        original
            .models
            .sync(
                &mut original.world,
                &mut original.assets,
                &mut donor,
                &library,
                &bodies,
                &settings
            )
            .unwrap()
    );
    original.interpolate(true).unwrap();
    for _ in 0..90 {
        original.step(0).unwrap();
    }
    for tick in 0..120 {
        original
            .step(if tick % 30 == 0 { ATTACK } else { 0 })
            .unwrap();
        if original.world.actors[original.world.player].actor.state
            == tenchu_core::tenchu2::actor::State::Attack
        {
            break;
        }
    }
    assert_eq!(
        original.world.actors[original.world.player].actor.state,
        tenchu_core::tenchu2::actor::State::Attack
    );
    let bytes = original.capture_state().unwrap();
    let mut restored = Session::load(&disc, 119, 1).unwrap();
    restored.restore_state(&bytes).unwrap();
    assert!(bytes == restored.capture_state().unwrap());
    assert_continuation(&mut original, &mut restored, 150);
    for session in [&mut original, &mut restored] {
        for _ in 0..120 {
            session.step(0).unwrap();
        }
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &bodies,
                    &Settings::default()
                )
                .unwrap()
        );
        assert!(
            session.world.actors[session.world.player]
                .actor
                .moveset
                .is_none()
        );
    }
    assert_continuation(&mut original, &mut restored, 60);
}

#[test]
#[ignore = "requires both discs; saved custom body geometry survives removal of its source folder"]
fn native_session_snapshot_embeds_custom_body_resources() {
    use crate::tenchu2::native::models;
    use crate::{
        settings::Settings,
        visuals::{appearance::library::Library, pack::Pack},
    };
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&game).unwrap();
    let mut donor = models::donor(&game).unwrap();
    let folder = std::env::temp_dir().join(format!(
        "tenchu-native-state-body-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&folder).unwrap();
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let bodies = Pack::bodies(folder.to_str().unwrap()).unwrap();
    let mut original = Session::load(&disc, 119, 17).unwrap();
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let settings = Settings {
        player_model: "pack:0000".into(),
        ..Default::default()
    };
    assert!(
        original
            .models
            .sync(
                &mut original.world,
                &mut original.assets,
                &mut donor,
                &library,
                &bodies,
                &settings
            )
            .unwrap()
    );
    let bytes = original.capture_state().unwrap();
    std::fs::remove_dir_all(&folder).unwrap();
    let mut restored = Session::load(&disc, 9, 1).unwrap();
    restored.restore_state(&bytes).unwrap();
    let saved = restored.models.custom.as_ref().unwrap().pack.clone();
    assert!(saved.models.values().any(|model| !model.is_empty()) && !saved.images.is_empty());
    assert!(postcard::to_allocvec(&bodies).unwrap() == postcard::to_allocvec(&saved).unwrap());
    assert!(
        !restored
            .models
            .sync(
                &mut restored.world,
                &mut restored.assets,
                &mut donor,
                &Library::default(),
                &saved,
                &settings
            )
            .unwrap()
    );
    assert_continuation(&mut original, &mut restored, 90);
}

#[test]
#[ignore = "requires both discs; imported stage scripts and audio clocks continue from a complete native session snapshot"]
fn native_session_snapshot_restores_imported_director() {
    use crate::tenchu2::native::gaisen;
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let source = gaisen::Source::open(
        path.as_ref(),
        0,
        0,
        1,
        tenchu_assets::script::Language::English,
    )
    .unwrap();
    let options = || Options {
        hero: 2,
        difficulty: 2,
        seed: 17,
        inventory: Default::default(),
    };
    let mut original = source.session(&disc, options(), false, 6000).unwrap();
    original.interpolate(true).unwrap();
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let bytes = original.capture_state().unwrap();
    let mut restored = source.session(&disc, options(), false, 6000).unwrap();
    restored.restore_state(&bytes).unwrap();
    assert!(bytes == restored.capture_state().unwrap());
    assert_continuation(&mut original, &mut restored, 180);
    let wrong = gaisen::Source::open(
        path.as_ref(),
        0,
        1,
        1,
        tenchu_assets::script::Language::English,
    )
    .unwrap();
    let mut wrong = wrong.session(&disc, options(), false, 6000).unwrap();
    let before = wrong.capture_state().unwrap();
    assert!(wrong.restore_state(&bytes).is_err());
    assert!(before == wrong.capture_state().unwrap());
    let mut native = Session::load(&disc, 9, 1).unwrap();
    let before = native.capture_state().unwrap();
    assert!(native.restore_state(&bytes).is_err());
    assert!(before == native.capture_state().unwrap());
}

#[test]
#[ignore = "requires TENCHU2_DISC; complete native world/resources/display snapshots continue all three heroes with live particles"]
fn native_resource_snapshots_preserve_simulation_and_display_continuation() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [9, 119, 191] {
        let mut original = Session::load(&disc, sequence, 17).unwrap();
        original.world.actors[original.world.player]
            .actor
            .invulnerable = true;
        for _ in 0..60 {
            original.step(0).unwrap();
        }
        let position = original.world.actors[original.world.player].actor.position;
        original
            .world
            .effects
            .explosion(
                position,
                0,
                &original.assets.resources.math,
                &mut original.world.random,
            )
            .unwrap();
        for _ in 0..3 {
            original.step(0).unwrap();
        }
        assert!(!original.frame.effects.sprites.is_empty());
        let before =
            postcard::to_allocvec(&(&original.world, &original.assets, &original.frame)).unwrap();
        let bytes = crate::savestate::encode(&(&original.world, &original.assets, &original.frame))
            .unwrap();
        let (world, assets, frame): (World, Assets, Frame) =
            crate::savestate::decode(&bytes).unwrap();
        assert!(postcard::to_allocvec(&(&world, &assets, &frame)).unwrap() == before);
        let mut restored = Session::from_parts(world, assets, original.bank.clone());
        restored.frame = frame;
        for tick in 0..180 {
            let buttons = match tick % 90 {
                0..30 => RIGHT,
                30..60 => UP,
                60 => ATTACK,
                _ => 0,
            };
            let a = original.step(buttons).unwrap();
            let b = restored.step(buttons).unwrap();
            assert_eq!(
                a.iter().map(|s| s.sound).collect::<Vec<_>>(),
                b.iter().map(|s| s.sound).collect::<Vec<_>>(),
                "audio sequence {sequence}, update {tick}",
            );
            assert!(
                postcard::to_allocvec(&original.world).unwrap()
                    == postcard::to_allocvec(&restored.world).unwrap(),
                "world sequence {sequence}, update {tick}",
            );
            assert!(
                postcard::to_allocvec(&original.frame).unwrap()
                    == postcard::to_allocvec(&restored.frame).unwrap(),
                "display sequence {sequence}, update {tick}",
            );
        }
        assert!(
            postcard::to_allocvec(&original.assets).unwrap()
                == postcard::to_allocvec(&restored.assets).unwrap(),
            "resource continuation sequence {sequence}",
        );
        eprintln!(
            "Native sequence {sequence}: {} bytes raw, {} bytes compressed; 180 continued updates match",
            before.len(),
            bytes.len(),
        );
    }
}
