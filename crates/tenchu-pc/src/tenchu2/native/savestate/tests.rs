use super::*;
use crate::source::DiscSource;

mod rewind;

fn start(sequence: usize) -> Playback<DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    Playback::load(disc, sequence, 2, 17, Some([0; 21]), None).unwrap()
}

fn renderer(playback: &Playback<DiscSource>, settings: &Settings) -> Renderer {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [640, 480],
        playback.vram(),
    ))
    .unwrap();
    renderer.configure(&super::super::preferences::graphics(settings));
    renderer
}

fn check_rewind(
    playback: &Playback<DiscSource>,
    drawing: &ContentDrawing,
    settings: &Settings,
    tools: &Tools,
) {
    let state =
        capture_rewind(playback, drawing, settings, tools, &mut Default::default()).unwrap();
    assert!(
        state.bytes().unwrap()
            == postcard::to_allocvec(&snapshot(playback, drawing, settings, tools).unwrap())
                .unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC and GPU; the host save boundary restores gameplay preferences/tools, retains current graphics and continues gameplay/cutscenes"]
fn native_savestates_restore_gameplay_preferences_and_continue() {
    for sequence in [9, 23] {
        let mut original = start(sequence);
        let mut settings = Settings {
            keep_enemies_active: true,
            wait_after_death: true,
            live_death_scene: true,
            circle_action: tenchu_core::input::CircleAction::Crouch,
            ..Default::default()
        };
        settings.bleeding.enabled = true;
        settings.bleeding.bandages = 3;
        settings.walking.enabled = true;
        settings.walking.threshold = 45;
        original.configure(&settings).unwrap();
        for _ in 0..30 {
            original.step(0).unwrap();
        }
        if let Content::Gameplay(session) = &mut original.content {
            session.world.controls.walking.as_mut().unwrap().held = true;
            session.step(tenchu_core::tenchu2::input::UP).unwrap();
        }
        let mut tools = Tools::default();
        tools.sync(&original.content, original.generation);
        tools.debug.actors_edited = true;
        let mut renderer = renderer(&original, &settings);
        let graphics = renderer.graphics().clone();
        let mut drawing = ContentDrawing::new(&original.content, &graphics, &mut renderer).unwrap();
        drawing
            .update(&original.content, &mut renderer, &tools.debug, &settings)
            .unwrap();
        let bytes = capture(&original, &drawing, &settings, &tools).unwrap();
        check_rewind(&original, &drawing, &settings, &tools);
        let source = original.capture_state().unwrap();
        let mut restored = start(119);
        let mut current = Settings::default();
        current.graphics.fog = !settings.graphics.fog;
        current.volume = 17;
        let graphics = current.graphics.clone();
        let mut restored_tools = Tools::default();
        restored_tools.debug.state_slot = 4;
        restored_tools.debug.click_kill = true;
        renderer.configure(&super::super::preferences::graphics(&current));
        let mut restored_drawing = restore(
            &mut restored,
            &bytes,
            &mut current,
            &mut restored_tools,
            &mut renderer,
        )
        .unwrap();
        assert_eq!(current.graphics, graphics);
        assert_eq!(current.volume, 17);
        assert_eq!(current.circle_action, settings.circle_action);
        assert_eq!(current.bleeding, settings.bleeding);
        assert_eq!(current.walking, settings.walking);
        assert!(
            current.keep_enemies_active && current.wait_after_death && current.live_death_scene
        );
        assert!(restored_tools.debug.actors_edited && !restored_tools.debug.click_kill);
        assert_eq!(restored_tools.debug.state_slot, 4);
        restored.configure(&current).unwrap();
        assert!(restored.capture_state().unwrap() == source);
        restored_drawing
            .update(
                &restored.content,
                &mut renderer,
                &restored_tools.debug,
                &current,
            )
            .unwrap();
        let state = restored.capture_state().unwrap();
        let prefs = current.clone();
        for bad in [bytes[..bytes.len() - 1].to_vec(), {
            let mut saved: State<campaign::state::Owned> =
                crate::savestate::decode(&bytes).unwrap();
            saved.signature.clear();
            crate::savestate::encode(&saved).unwrap()
        }] {
            let generation = restored.generation;
            let error = restore(
                &mut restored,
                &bad,
                &mut current,
                &mut restored_tools,
                &mut renderer,
            )
            .err()
            .expect("damaged state was accepted");
            assert!(
                !error.drawing_changed,
                "rejected file invalidated current presentation"
            );
            assert_eq!(restored.generation, generation);
            assert_eq!(current, prefs);
            assert!(restored.capture_state().unwrap() == state);
        }
        for _ in 0..90 {
            original.step(0).unwrap();
            restored.step(0).unwrap();
        }
        assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    }
}

#[test]
#[ignore = "requires both discs and GPU; loaded imported body, moveset, equipment and voice continue without reloading their source gallery"]
fn native_savestates_keep_embedded_overrides_with_an_empty_gallery() {
    use crate::{tenchu2::native::models, visuals::appearance::library::Library};
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = models::donor(&game).unwrap();
    let bodies = std::sync::Arc::default();
    let mut original = start(9);
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let mut settings = Settings {
        player_model: "disc:1".into(),
        model_moveset: true,
        moveset_model: "tenchu2-jp:ayamea".into(),
        weapon_model: "tenchu2-jp:yorojyu".into(),
        voice_model: "tenchu2-jp:kagami".into(),
        ..Default::default()
    };
    let Content::Gameplay(session) = &mut original.content else {
        unreachable!()
    };
    session
        .models
        .sync(
            &mut session.world,
            &mut session.assets,
            &mut donor,
            &library,
            &bodies,
            &settings,
        )
        .unwrap();
    let mut renderer = renderer(&original, &settings);
    let graphics = renderer.graphics().clone();
    let drawing = ContentDrawing::new(&original.content, &graphics, &mut renderer).unwrap();
    let mut tools = Tools::default();
    let bytes = capture(&original, &drawing, &settings, &tools).unwrap();
    check_rewind(&original, &drawing, &settings, &tools);
    let mut restored = start(119);
    let mut drawing = restore(
        &mut restored,
        &bytes,
        &mut settings,
        &mut tools,
        &mut renderer,
    )
    .unwrap();
    rewind::check_seek(
        &mut restored,
        &mut drawing,
        &mut settings,
        &mut tools,
        &mut renderer,
    );
    let expected = restored.capture_state().unwrap();
    let Content::Gameplay(session) = &mut restored.content else {
        unreachable!()
    };
    assert!(
        !session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &Library::default(),
                &bodies,
                &settings
            )
            .unwrap()
    );
    assert!(session.models.voice.is_some() && session.models.weapon_voice.is_some());
    assert!(expected == restored.capture_state().unwrap());
    for _ in 0..90 {
        original.step(0).unwrap();
        restored.step(0).unwrap();
    }
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
}

#[test]
#[ignore = "requires both discs and GPU; persisted native T1-stage states reopen after results without losing director or retry context"]
fn native_savestates_restore_imported_stages_from_disk() {
    use crate::tenchu2::native::gaisen;
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    let load = || {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let source = gaisen::Source::open(
            std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
            0,
            0,
            2,
            tenchu_assets::script::Language::English,
        )
        .unwrap();
        Playback::gaisen(disc, source, 2, 2, 17, Some([0; 21]), 6000).unwrap()
    };
    let mut original = load();
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let mut settings = Settings::default();
    let mut tools = Tools::default();
    let mut renderer = renderer(&original, &settings);
    let graphics = renderer.graphics().clone();
    let mut drawing = ContentDrawing::new(&original.content, &graphics, &mut renderer).unwrap();
    drawing
        .update(&original.content, &mut renderer, &tools.debug, &settings)
        .unwrap();
    let bytes = capture(&original, &drawing, &settings, &tools).unwrap();
    check_rewind(&original, &drawing, &settings, &tools);
    let root = std::env::temp_dir().join(format!(
        "tenchu-native-state-file-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join("t2-jp-slot-1.tstate");
    crate::storage::atomic_write(&path, &bytes).unwrap();
    let mut restored = load();
    let Content::Gameplay(session) = &mut restored.content else {
        unreachable!()
    };
    session.world.mission.phase = Phase::Finished {
        outcome: Outcome::Arrived,
        finished_at: 180,
    };
    restored.step(0).unwrap();
    assert!(matches!(restored.content, Content::Results(_)));
    let mut drawing = restore(
        &mut restored,
        &crate::savestate::read(&path).unwrap(),
        &mut settings,
        &mut tools,
        &mut renderer,
    )
    .unwrap();
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    drawing
        .update(&restored.content, &mut renderer, &tools.debug, &settings)
        .unwrap();
    rewind::check_seek(
        &mut restored,
        &mut drawing,
        &mut settings,
        &mut tools,
        &mut renderer,
    );
    for _ in 0..90 {
        original.step(0).unwrap();
        restored.step(0).unwrap();
    }
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    std::fs::remove_dir_all(root).unwrap();
}
